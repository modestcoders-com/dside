import { watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { settings } from "../stores/settings";

/**
 * The window title carries the project.
 *
 * Several DS·IDE windows can be open at once — one per project is the whole point of the rail.
 * The task bar and the window switcher list them by title, so "DS·IDE" twice is unusable:
 * "DS·IDE — TallyGrip" and "DS·IDE — Website" are not.
 */
export function trackWindowTitle(): void {
  watch(
    () => settings.workspace,
    (workspace) => {
      void invoke("set_project_title", { workspace: workspace ?? "" }).catch(() => undefined);
    },
    { immediate: true },
  );
}
