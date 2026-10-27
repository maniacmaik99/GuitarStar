// Typed wrapper for the Linux/Proton setup command in src-tauri/src/commands.rs.
import { invoke } from "@tauri-apps/api/core";
import type { AsioDriver } from "$lib/api";

export interface LinuxSetupStatus {
  proton_prefix: string | null;
  asio_drivers: AsioDriver[];
  /** wineasio is registered for 32-bit programs in the Proton prefix. */
  wineasio_registered: boolean;
  wineasio_dll_in_prefix: boolean;
  /** Launch option line for the mods in the game folder; null if none need one. */
  launch_options: string | null;
  /** False when no Steam localconfig.vdf could be read. */
  launch_options_checked: boolean;
  current_launch_options: string | null;
  /** DLL overrides (or "%command%") missing from the current launch options. */
  missing: string[];
}

/** Null on Windows or before a game folder is chosen. */
export const linuxSetupStatus = () => invoke<LinuxSetupStatus | null>("linux_setup_status");
