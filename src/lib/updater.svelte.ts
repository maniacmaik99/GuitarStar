// Shared update state for the banner and the settings tab.
import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { errorText } from "$lib/api";

export const updater = $state({
  version: null as string | null,
  update: null as Update | null,
  checking: false,
  /** Result of the last check started by the user; null while none finished. */
  upToDate: null as boolean | null,
  checkError: null as string | null,
  /** Download progress in percent, or -1 while the total size is unknown. */
  progress: null as number | null,
  installError: null as string | null,
  dismissed: false,
});

export async function loadVersion() {
  updater.version = await getVersion().catch(() => null);
}

/** `quiet` hides errors, for the automatic check at startup (e.g. offline). */
export async function checkForUpdate(quiet = false) {
  updater.checking = true;
  updater.checkError = null;
  try {
    updater.update = await check();
    updater.upToDate = !updater.update;
    updater.dismissed = false;
  } catch (e) {
    if (!quiet) updater.checkError = errorText(e);
  } finally {
    updater.checking = false;
  }
}

export async function installUpdate() {
  const update = updater.update;
  if (!update) return;
  updater.installError = null;
  let total = 0;
  let done = 0;
  updater.progress = 0;
  try {
    await update.downloadAndInstall((event) => {
      if (event.event === "Started") total = event.data.contentLength ?? 0;
      else if (event.event === "Progress") {
        done += event.data.chunkLength;
        updater.progress = total ? Math.round((done / total) * 100) : -1;
      }
    });
    await relaunch();
  } catch (e) {
    updater.installError = errorText(e);
    updater.progress = null;
  }
}
