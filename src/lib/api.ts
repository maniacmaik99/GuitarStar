// Typed wrappers around the Rust commands in src-tauri/src/commands.rs.
import { invoke } from "@tauri-apps/api/core";
import { locale } from "$lib/i18n.svelte";

export type OutputMode = "asio" | "windows" | "ask";

export interface AudioConfig {
  profile_id: string | null;
  driver: string;
  output_mode: OutputMode;
  output_only: boolean;
  output_base_channel: number;
  alt_base_channel: number | null;
  input_channel: number;
  second_input_channel: number | null;
  buffer_size_mode: string;
  custom_buffer_size: number | null;
  enable_ref_count_hack: boolean | null;
}

export interface DeviceProfile {
  id: string;
  name: string;
  driver: string;
  output_only: boolean;
  output_base_channel: number;
  alt_base_channel: number | null;
  input_channel: number;
  second_input_channel: number | null;
  buffer_size_mode: string;
  custom_buffer_size: number | null;
  enable_ref_count_hack: boolean | null;
  notes: string | null;
  source: string | null;
  custom: boolean;
}

export interface AsioDriver {
  name: string;
  has_32bit: boolean;
  has_64bit: boolean;
}

export interface GameInfo {
  path: string;
  valid: boolean;
  steam_version: boolean;
  has_rocksmith_ini: boolean;
  proton_prefix: string | null;
  problems: string[];
}

export interface GameSettings {
  screen_width: number | null;
  screen_height: number | null;
  fullscreen: number | null;
  exclusive_mode: boolean | null;
  ultra_low_latency: boolean | null;
  latency_buffer: number | null;
}

export interface InstalledComponent {
  version: string;
  files: string[];
  installed_at: number;
}

export interface CachedSource {
  version: string;
  asset_url: string;
  sha256: string;
  downloaded_at: number;
}

export interface ComponentStatus {
  id: string;
  name: string;
  description: string;
  repo: string | null;
  bundled_version: string | null;
  installed: InstalledComponent | null;
  files_present: boolean;
  conflicts: string[];
  cached: CachedSource[];
}

export interface ReleaseInfo {
  version: string;
  published: string | null;
  page_url: string;
  asset_name: string;
  asset_url: string;
  asset_size: number;
  digest: string | null;
  prerelease: boolean;
}

export interface SnapshotSummary {
  id: string;
  created: number;
  label: string;
  file_count: number;
}

export interface AppState {
  game_dir: string | null;
  installed: Record<string, InstalledComponent>;
  audio: AudioConfig | null;
  dpi_override: boolean;
  keep_visible: boolean;
  hide_ads: boolean;
  dll_log: boolean;
}

export interface Overview {
  platform: string;
  state: AppState;
  game: GameInfo | null;
  settings: GameSettings | null;
  dpi_override: boolean | null;
  components: ComponentStatus[];
  snapshots: SnapshotSummary[];
}

export interface MonitorSize {
  name: string | null;
  width: number;
  height: number;
  scale_factor: number;
}

export interface CdlcEntry {
  file_name: string;
  artist: string | null;
  title: string;
  version: string | null;
  size: number;
  enabled: boolean;
  modified: number;
  /** Normalised artist + title; equal keys mean probably several versions of one song. */
  song_key: string;
}

export interface SongSource {
  cf_id: string;
  downloaded_at: number;
  source_url_host: string | null;
}

export type BulkAction = "enable" | "disable" | "delete";

export interface BulkResult {
  done: string[];
  failed: [string, string][];
}

export interface CdlcOverview {
  songs: CdlcEntry[];
  enabler_installed: boolean;
  cherub_rock_installed: boolean | null;
}

export interface ImportResult {
  imported: string[];
  skipped: [string, string][];
}

export interface DownloadReport {
  file_name: string;
  result: ImportResult | null;
  error: string | null;
}

export interface OutputDevice {
  name: string;
  device: string | null;
  bluetooth: boolean;
}

export interface RsModsStatus {
  installed: boolean;
  gui: string | null;
  cache_modified: boolean | null;
}

/** An area of the main window in CSS pixels. */
export interface Bounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export const api = {
  overview: () => invoke<Overview>("overview"),
  detectGames: () => invoke<string[]>("detect_games"),
  setGameDir: (path: string) => invoke<GameInfo>("set_game_dir", { path }),
  saveGameSettings: (settings: GameSettings) => invoke<void>("save_game_settings", { settings }),
  listProfiles: () => invoke<DeviceProfile[]>("list_profiles"),
  saveProfile: (profile: DeviceProfile) => invoke<void>("save_profile", { profile }),
  listAsioDrivers: () => invoke<AsioDriver[]>("list_asio_drivers"),
  applyAudioConfig: (config: AudioConfig) => invoke<void>("apply_audio_config", { config }),
  componentReleases: (id: string) => invoke<ReleaseInfo[]>("component_releases", { id }),
  installComponent: (id: string, version: string) => invoke<void>("install_component", { id, version }),
  uninstallComponent: (id: string) => invoke<void>("uninstall_component", { id }),
  restoreSnapshot: (id: string) => invoke<void>("restore_snapshot", { id }),
  restoreOriginal: () => invoke<void>("restore_original"),
  monitorSizes: () => invoke<MonitorSize[]>("monitor_sizes"),
  setDpiOverride: (enabled: boolean) => invoke<void>("set_dpi_override", { enabled }),
  setKeepVisible: (enabled: boolean) => invoke<void>("set_keep_visible", { enabled }),
  setDllLog: (enabled: boolean) => invoke<void>("set_dll_log", { enabled }),
  cdlcOverview: () => invoke<CdlcOverview>("cdlc_overview"),
  cdlcImport: (paths: string[]) => invoke<ImportResult>("cdlc_import", { paths }),
  cdlcSetEnabled: (name: string, enabled: boolean) => invoke<void>("cdlc_set_enabled", { name, enabled }),
  cdlcDelete: (name: string) => invoke<void>("cdlc_delete", { name }),
  cdlcBulk: (names: string[], action: BulkAction) => invoke<BulkResult>("cdlc_bulk", { names, action }),
  cdlcSources: () => invoke<Record<string, SongSource>>("cdlc_sources"),
  customsforgeOpenUrl: (url: string) => invoke<void>("customsforge_open_url", { url }),
  installCdlcEnabler: () => invoke<void>("install_cdlc_enabler"),
  openCustomsforge: (b: Bounds) => invoke<void>("open_customsforge", { ...b }),
  customsforgeBounds: (b: Bounds) => invoke<void>("customsforge_bounds", { ...b }),
  hideCustomsforge: () => invoke<void>("hide_customsforge"),
  setHideAds: (enabled: boolean) => invoke<void>("set_hide_ads", { enabled }),
  defaultOutputDevice: () => invoke<OutputDevice | null>("default_output_device"),
  rsmodsStatus: () => invoke<RsModsStatus>("rsmods_status"),
  installRsmods: () => invoke<string>("install_rsmods"),
  openRsmodsGui: () => invoke<void>("open_rsmods_gui"),
  uninstallRsmods: () => invoke<void>("uninstall_rsmods"),
  customsforgeNavigate: (action: "back" | "forward" | "reload" | "home") =>
    invoke<void>("customsforge_navigate", { action }),
};

/** Provided by the page: runs an action with busy/error handling and refreshes the overview. */
export type Runner = (label: string, action: () => Promise<unknown>, done?: string) => Promise<boolean>;

export function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : JSON.stringify(e);
}

export function formatDate(unixSeconds: number): string {
  return new Date(unixSeconds * 1000).toLocaleString(locale(), { dateStyle: "medium", timeStyle: "short" });
}
