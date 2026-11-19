import { invoke } from "@tauri-apps/api/core";

/**
 * Waits until the backend has finished `setup` — the moment `Db` and the keyring are managed.
 *
 * On an older Windows machine the webview asked first: commands that carry no state answered
 * (working folder, editor list, title) while the ones that need a database or the keyring failed.
 * An empty answer looked like a fresh installation, and the interface then wrote its defaults over
 * the stored values. The wait removes that race; the retries elsewhere in the app stay, so a bug
 * in this flag cannot cost the configuration again.
 *
 * Not fatal, ever: if the flag never arrives, this logs and lets the app boot anyway.
 */
export async function waitUntilReady(): Promise<void> {
  const deadline = Date.now() + 10_000;
  while (Date.now() < deadline) {
    try {
      if (await invoke<boolean>("app_ready")) return;
    } catch {
      // The command may not be registered yet, or the IPC channel is not up. Both are answers
      // of the form "not yet", so the loop simply asks again.
    }
    await new Promise((resolve) => window.setTimeout(resolve, 25));
  }
  void invoke("log_message", { text: "startup: app_ready never arrived" }).catch(() => undefined);
}
