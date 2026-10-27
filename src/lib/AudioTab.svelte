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
  import { t } from "$lib/i18n.svelte";
  import LinuxSetup from "$lib/LinuxSetup.svelte";

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
    run(t("audio.applying"), () => api.applyAudioConfig($state.snapshot(form)), t("audio.applied"));
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
    const ok = await run(t("audio.savingProfile"), () => api.saveProfile(profile), t("audio.profileSaved", { name }));
    if (ok) {
      profiles = await api.listProfiles();
      form.profile_id = id;
      profileName = "";
    }
  }
</script>

{#if ov.platform === "linux"}
  <LinuxSetup {ov} />
{/if}

{#if !rsAsioInstalled}
  <section class="card">
    <h2>{t("audio.notInstalledTitle")}</h2>
    <p>{t("audio.notInstalledText")}</p>
    <button class="primary" onclick={gotoComponents}>{t("audio.gotoComponents")}</button>
  </section>
{:else if !loaded}
  <p class="muted">{t("audio.searchingDrivers")}</p>
{:else}
  {#if drivers.length === 0}
    <section class="card">
      <h2>{t("audio.noDriverTitle")}</h2>
      {#if ov.platform === "linux"}
        <p>{t("audio.linuxHint")}</p>
      {:else}
        <p>{t("audio.noDriverText")}</p>
      {/if}
    </section>
  {/if}

  <section class="card">
    <h2>{t("audio.device")}</h2>
    <div class="field">
      <label for="profile">{t("audio.profile")}</label>
      <select id="profile" value={form.profile_id ?? ""} onchange={(e) => applyProfile(e.currentTarget.value)}>
        <option value="">{t("audio.customSettings")}</option>
        {#if suitable.length}
          <optgroup label={t("audio.matchingProfiles")}>
            {#each suitable as p}<option value={p.id}>{p.name}</option>{/each}
          </optgroup>
        {/if}
        <optgroup label={t("audio.allProfiles")}>
          {#each profiles as p}<option value={p.id}>{p.name}{p.custom ? t("audio.ownSuffix") : ""}</option>{/each}
        </optgroup>
      </select>
    </div>
    {#if selectedProfile?.notes}
      <p class="muted">{selectedProfile.notes}</p>
    {/if}
    <div class="field">
      <label for="driver">{t("audio.driver")}</label>
      <select id="driver" bind:value={form.driver}>
        {#if form.driver && !selectedDriver}
          <option value={form.driver}>{t("audio.driverMissing", { name: form.driver })}</option>
        {/if}
        {#each drivers as d}
          <option value={d.name}>{d.name}{d.has_32bit ? "" : t("audio.only64Suffix")}</option>
        {/each}
      </select>
    </div>
    {#if selectedDriver && !selectedDriver.has_32bit}
      <div class="hint warn">{t("audio.only64Hint")}</div>
    {/if}
    <div class="field">
      <label for="inch">{t("audio.inputChannel")}</label>
      <input id="inch" type="number" min="0" max="64" bind:value={form.input_channel} disabled={form.output_only} />
    </div>
    <div class="field">
      <label for="outch">{t("audio.outputChannel")}</label>
      <input id="outch" type="number" min="0" max="64" bind:value={form.output_base_channel} />
    </div>
    <div class="field">
      <label for="buf">{t("audio.bufferSize")}</label>
      <div class="row">
        <select id="buf" bind:value={form.buffer_size_mode}>
          <option value="driver">{t("audio.bufferDriver")}</option>
          <option value="host">{t("audio.bufferHost")}</option>
          <option value="custom">{t("audio.bufferCustom")}</option>
        </select>
        {#if form.buffer_size_mode === "custom"}
          <input type="number" min="16" max="2048" step="16" bind:value={form.custom_buffer_size} />
          <span class="muted">{t("audio.bufferHint")}</span>
        {/if}
      </div>
    </div>
  </section>

  <section class="card">
    <h2>{t("audio.output")}</h2>
    <label class="option">
      <input type="radio" bind:group={form.output_mode} value="asio" />
      <div>
        <strong>{t("audio.outAsio")}</strong>
        <div class="muted">{t("audio.outAsioText")}</div>
      </div>
    </label>
    <label class="option">
      <input type="radio" bind:group={form.output_mode} value="windows" />
      <div>
        <strong>{t("audio.outWindows")}</strong>
        <div class="muted">{t("audio.outWindowsText")}</div>
      </div>
    </label>
    <label class="option">
      <input type="radio" bind:group={form.output_mode} value="ask" />
      <div>
        <strong>{t("audio.outAsk")}</strong>
        <div class="muted">{t("audio.outAskText")}</div>
      </div>
    </label>
    {#if form.output_mode !== "asio" && windowsOutput}
      <div class="device" class:warn={windowsOutput.bluetooth}>
        {t("audio.windowsOutput")} <strong>{windowsOutput.name}</strong>{windowsOutput.device ? ` (${windowsOutput.device})` : ""}
        <button class="link" onclick={async () => (windowsOutput = await api.defaultOutputDevice())}>{t("audio.recheck")}</button>
        {#if windowsOutput.bluetooth}
          <p>{t("audio.bluetoothHint")}</p>
        {/if}
      </div>
    {/if}
  </section>

  {#if ov.settings?.exclusive_mode && ov.state.audio && ov.state.audio.output_mode !== "asio"}
    <div class="hint">{t("audio.exclusiveHint")}</div>
  {/if}

  <div class="row">
    <button class="primary" disabled={busy || !form.driver} onclick={save}>{t("audio.apply")}</button>
    <span class="muted">{t("audio.applyHint")}</span>
  </div>

  <section class="card" style="margin-top: 16px">
    <h2>{t("audio.saveProfileTitle")}</h2>
    <p class="muted">{t("audio.saveProfileText")}</p>
    <div class="row">
      <input placeholder={t("audio.profileNamePlaceholder")} bind:value={profileName} />
      <button disabled={busy || !profileName.trim() || !form.driver} onclick={saveAsProfile}>{t("common.save")}</button>
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
