import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

export interface AvailableUpdate {
  latest: string;
  current: string;
  url: string;
}

/**
 * The newer release the backend found, if any. `null` means either "nothing newer" or "could not
 * be checked" — the interface treats both the same, because a failed check is not a problem the
 * user can act on.
 */
export const updateAvailable = ref<AvailableUpdate | null>(null);

interface UpdateInfo {
  current: string;
  latest: string;
  url: string;
  available: boolean;
}

/**
 * Asked once per start, after the window is up: a network call must never be the reason a window
 * takes longer to appear. The backend caches its answer for six hours, so a remount does not ask
 * GitHub again.
 */
export async function checkForUpdate(): Promise<void> {
  try {
    const info = await invoke<UpdateInfo | null>("update_check", {});
    updateAvailable.value =
      info && info.available
        ? { latest: info.latest, current: info.current, url: info.url }
        : null;
  } catch {
    // No network, a rate limit, an API change — all of it ends in "no answer".
    updateAvailable.value = null;
  }
}

/** The release page, in the system browser: that is where an installer can be downloaded. */
export async function openUpdatePage(): Promise<void> {
  const target = updateAvailable.value?.url;
  if (!target) return;
  await openUrl(target).catch(() => undefined);
}
