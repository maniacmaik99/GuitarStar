// Typed wrappers for the save backup commands in src-tauri/src/commands.rs.
import { invoke } from "@tauri-apps/api/core";

export interface SaveFolder {
  /** Steam account id (folder name under userdata). */
  account: string;
  path: string;
  file_count: number;
  size: number;
  /** Unix seconds of the newest file; null for an empty folder. */
  modified: number | null;
}

export interface SaveBackup {
  id: string;
  created: number;
  /** Text in the language at creation time; shown when `label_key` is unknown. */
  label: string;
  label_key?: string;
  label_args?: Record<string, string>;
  account: string;
  automatic: boolean;
  source: string;
  size: number;
  files: { path: string; size: number; sha256: string }[];
}

export const saves = {
  folders: () => invoke<SaveFolder[]>("save_folders"),
  backups: () => invoke<SaveBackup[]>("save_backups"),
  backup: (label?: string) => invoke<SaveBackup[]>("backup_saves", { label: label ?? null }),
  /** The daily automatic backup; resolves to true when one was made. */
  autoBackup: () => invoke<boolean>("auto_backup_saves"),
  restore: (id: string) => invoke<void>("restore_save_backup", { id }),
  remove: (id: string) => invoke<void>("delete_save_backup", { id }),
  openFolder: (account?: string) => invoke<void>("open_save_folder", { account: account ?? null }),
};
