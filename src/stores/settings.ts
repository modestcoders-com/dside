import { computed, reactive, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { loadJson } from "../lib/persist";
import type { AppMode, Appearance, CommandPolicy, PermissionMode } from "../types";

export interface SettingsState {
  mode: AppMode;
  appearance: Appearance;
  showInMenuBar: boolean;
  launchOnStartup: boolean;
  desktopNotifications: boolean;
  joinEarlyAccess: boolean;
  /** read = nur lesen, ask = vor Änderungen fragen, full = Vollzugriff. */
  permission: PermissionMode;
  /** Allowlist for the companion's bash tool; the user's own terminal is not affected. */
  commandPolicy: CommandPolicy;
  allowedCommands: string[];
  /** Extra folders the companion's commands search, so a tool outside the system PATH is found
   *  by its name. Empty by default: the allowlist still names programs, this only widens where. */
  commandDirs: string[];
  /** Keep write/edit inside the workspace even in full access. */
  workspaceOnly: boolean;
  /** Let the work area follow files and pages the companion opens. */
  followCompanion: boolean;
  pinnedEditorId: string | null;
  /** The active project: the folder the tools read and write in. */
  workspace: string;
  /** The folder whose subfolders are offered as projects. Only ever listed, never written to. */
  projectsRoot: string;
  baseUrl: string;
  model: string;
  displayName: string;
  fontSize: number;
  /** The user's own addition to the partner's instructions, verbatim. */
  partnerPrompt: string;
  /** Per-command overrides of the keyboard defaults; an absent id means the defaults apply. */
  shortcuts: Record<string, string[]>;
  /** Let the partner read and fill forms in browser windows the user opened. */
  browserControl: boolean;
  /** Closing the window keeps DS.IDE running in the tray, so reminders can still fire. */
  keepInBackground: boolean;
  /** How hard the model should think, on the provider's own scale. Empty = provider default. */
  reasoningEffort: string;
  /** How long a question or an approval waits before the turn gives up, in minutes. */
  askTimeoutMinutes: number;
  /** A second folder scanned for plugins, next to the app's own. Empty = only the app's own. */
  pluginRoot: string;
  /** Plugin ids enabled per folder. The same plugin can be on in one project and off in
   *  another; turning it off never touches its files. */
  enabledPlugins: Record<string, string[]>;
  /** Set once the first-start dialog has been answered or dismissed. */
  onboarded: boolean;
}

/**
 * Which model a mode starts with — both ids are the ones the provider reports. Everyday work
 * gets the Pro model, coding the faster Flash one. Changing these two lines changes the
 * pre-selection for both modes; a model the user picks by hand is never overwritten.
 */
const MODEL_BY_MODE: Record<AppMode, string> = {
  everyday: "deepseek-v4-pro",
  coding: "deepseek-flash",
};

export function defaultModelFor(mode: AppMode): string {
  return MODEL_BY_MODE[mode] ?? MODEL_BY_MODE.coding;
}

/** The companion's default shell allowlist: build tools plus read-only shell commands.
 * It is a policy, not an OS sandbox; a path or a shell builtin can still do more. */
/** The list shipped before the text tools were added — used only to recognise an untouched
 *  list and move it forward. Kept verbatim; do not edit. */
const PREVIOUS_DEFAULT_COMMANDS = [
  "pnpm", "npm", "npx", "yarn", "bun", "node", "deno",
  "cargo", "rustc", "rustup", "git",
  "python", "python3", "py", "pip", "pytest", "uv", "poetry",
  "dotnet", "go", "java", "javac", "mvn", "gradle",
  "make", "cmake", "gcc", "g++", "clang", "tsc", "tsx", "vite", "vitest",
  "jest", "eslint", "prettier", "ruff", "black", "mypy", "sqlite3",
  "ls", "dir", "pwd", "cd", "cat", "type", "head", "tail", "grep", "rg", "find",
  "where", "which", "echo", "sort", "uniq", "wc", "diff", "cmp", "file", "stat",
  "du", "df", "tree", "uname", "ver", "whoami", "env", "printenv", "date", "sleep",
];

/** The list 0.6.7 shipped (the text tools, no `touch`/`mkdir`). Kept verbatim for the same
 *  reason as the one above: it recognises an untouched list from that release. */
const PREVIOUS_TEXT_TOOL_COMMANDS = [
  "pnpm", "npm", "npx", "yarn", "bun", "node", "deno",
  "cargo", "rustc", "rustup", "git",
  "python", "python3", "py", "pip", "pytest", "uv", "poetry",
  "dotnet", "go", "java", "javac", "mvn", "gradle",
  "make", "cmake", "gcc", "g++", "clang", "tsc", "tsx", "vite", "vitest",
  "jest", "eslint", "prettier", "ruff", "black", "mypy", "sqlite3",
  "sed", "awk", "test", "jq", "xxd", "base64", "sha256sum", "md5sum",
  "ls", "dir", "pwd", "cd", "cat", "type", "head", "tail", "grep", "rg", "find",
  "where", "which", "echo", "sort", "uniq", "wc", "diff", "cmp", "file", "stat",
  "du", "df", "tree", "uname", "ver", "whoami", "env", "printenv", "date", "sleep",
];

export const defaultAllowedCommands = [
  "pnpm", "npm", "npx", "yarn", "bun", "node", "deno",
  "cargo", "rustc", "rustup", "git",
  "python", "python3", "py", "pip", "pytest", "uv", "poetry",
  "dotnet", "go", "java", "javac", "mvn", "gradle",
  "make", "cmake", "gcc", "g++", "clang", "tsc", "tsx", "vite", "vitest",
  "jest", "eslint", "prettier", "ruff", "black", "mypy", "sqlite3",
  // Reading and shaping text. `sed`/`awk` are in every shell session; leaving them out made
  // the companion rewrite commands into `python3 -c` one-liners, which is worse in every way.
  // `cmp` already stands in the group below — the list is rendered one chip per entry, so a
  // second mention would be a second chip with the same key.
  "sed", "awk", "test", "jq", "xxd", "base64", "sha256sum", "md5sum",
  // Creating a scratch directory or an empty file is harmless and needs no shell trick; `touch`
  // and `mkdir -p` change nothing that exists. `rm` and `cp` stay out on purpose — one deletes,
  // one overwrites — and anyone who wants them adds them under Policies.
  "touch", "mkdir",
  "ls", "dir", "pwd", "cd", "cat", "type", "head", "tail", "grep", "rg", "find",
  "where", "which", "echo", "sort", "uniq", "wc", "diff", "cmp", "file", "stat",
  "du", "df", "tree", "uname", "ver", "whoami", "env", "printenv", "date", "sleep",
];

/** Order-insensitive comparison: a list is a set here. */
function sameCommands(left: string[], right: string[]): boolean {
  if (left.length !== right.length) return false;
  const set = new Set(left);
  return right.every((entry) => set.has(entry));
}

/** The API key is deliberately NOT here: it lives in the OS keyring (see secret.rs). */
const PERSISTED_KEYS: (keyof SettingsState)[] = [
  "mode",
  "appearance",
  "showInMenuBar",
  "launchOnStartup",
  "desktopNotifications",
  "joinEarlyAccess",
  "permission",
  "commandPolicy",
  "allowedCommands",
  "commandDirs",
  "workspaceOnly",
  "followCompanion",
  "pinnedEditorId",
  "workspace",
  "projectsRoot",
  "baseUrl",
  "model",
  "displayName",
  "fontSize",
  "partnerPrompt",
  "shortcuts",
  "browserControl",
  "keepInBackground",
  "reasoningEffort",
  "askTimeoutMinutes",
  "pluginRoot",
  "enabledPlugins",
  "onboarded",
];

const LEGACY_KEY = "ds-ide.settings";

const defaults: SettingsState = {
  mode: "everyday",
  appearance: "dark",
  showInMenuBar: true,
  launchOnStartup: true,
  desktopNotifications: false,
  joinEarlyAccess: false,
  permission: "ask",
  commandPolicy: "allowlist",
  allowedCommands: [...defaultAllowedCommands],
  commandDirs: [],
  workspaceOnly: true,
  followCompanion: true,
  pinnedEditorId: null,
  workspace: "",
  projectsRoot: "",
  baseUrl: "https://api.deepseek.com",
  model: MODEL_BY_MODE.everyday,
  displayName: "",
  fontSize: 14,
  partnerPrompt: "",
  shortcuts: {},
  browserControl: false,
  keepInBackground: true,
  reasoningEffort: "",
  // Five minutes is a guess about somebody's attention; the field exists so it can be wrong.
  askTimeoutMinutes: 5,
  pluginRoot: "",
  enabledPlugins: {},
  onboarded: false,
};

export const settings = reactive<SettingsState>({ ...defaults });

/**
 * Older installations stored a single boolean. `true` was full access; `false` was the
 * original deny-everything mode. The middle "ask" stage did not exist, so it is only the
 * default for new installations.
 */
function permissionFromLegacy(stored: Record<string, unknown>): PermissionMode {
  const permission = stored.permission;
  if (permission === "read" || permission === "ask" || permission === "full") {
    return permission;
  }
  if (stored.fullAuthorization === true) return "full";
  if (stored.fullAuthorization === false) return "read";
  return "ask";
}

/** True only while the app has no stored settings at all; the dialog clears it. */
export const firstRun = ref(false);

let loaded = false;
let timer: number | undefined;
/** The stored settings could not be read. Different from "there are none". */
export const settingsUnreadable = ref(false);

/**
 * What was last written successfully, so a flush writes only what changed. Without it every
 * flush rewrote all twenty keys — and a flush on a window losing focus would be a write storm.
 */
const written: Record<string, string> = {};

async function persistNow(): Promise<void> {
  for (const key of PERSISTED_KEYS) {
    const value = JSON.stringify(settings[key]);
    if (written[key] === value) continue;
    try {
      await invoke("settings_set", { key, value });
      written[key] = value;
    } catch (error) {
      // Not silent any more: a write that fails on one platform and not another is exactly the
      // kind of thing nobody can investigate without a line in the log.
      void invoke("log_message", { text: `settings: writing ${key} failed: ${error}` }).catch(
        () => undefined,
      );
    }
  }
}

/**
 * Writes pending changes now instead of waiting for the debounce. Onboarding calls this: the
 * first run decides what the next start sees, and 400 ms was long enough for a quick quit to
 * lose it — which is one way "the dialog comes back every time" happens.
 */
export async function flushSettings(): Promise<void> {
  if (!loaded) return;
  if (timer) {
    window.clearTimeout(timer);
    timer = undefined;
  }
  await persistNow();
}

function schedulePersist(): void {
  if (!loaded) return;
  if (timer) window.clearTimeout(timer);
  timer = window.setTimeout(() => {
    timer = undefined;
    void persistNow();
  }, 400);
}

function apply(stored: Record<string, unknown>): void {
  const target = settings as unknown as Record<string, unknown>;
  for (const key of PERSISTED_KEYS) {
    if (!(key in stored)) continue;
    target[key] = stored[key];
  }
}

/**
 * Reads the stored settings, retrying a few times.
 *
 * A single failed read used to be indistinguishable from an empty database, so the app showed
 * the first-start dialog again on every start — even though the settings were there and an API
 * key was remembered. That happened on Windows, where the boot read may arrive before the IPC
 * channel is ready. Retrying costs a fraction of a second and removes the race; the distinction
 * between "failed" and "empty" is what matters, because one of them must never look like the
 * other.
 */
async function readStored(): Promise<Record<string, string> | null> {
  let lastError = "";
  // Twelve attempts, growing to half a second — about four seconds in total. The reason is not
  // politeness: on an older Windows machine the webview called before the backend had managed its
  // state, and one immediate failure was enough to turn the app into a fresh installation. A slow
  // start must cost seconds, not the whole configuration.
  for (let attempt = 0; attempt < 12; attempt += 1) {
    try {
      return await invoke<Record<string, string>>("settings_all");
    } catch (error) {
      lastError = String(error);
      await new Promise((resolve) =>
        window.setTimeout(resolve, Math.min(500, 80 + attempt * 60)),
      );
    }
  }
  void invoke("log_message", {
    text: `settings: could not be read after twelve attempts: ${lastError}`,
  }).catch(() => undefined);
  return null;
}

/**
 * Settings live in the SQLite `settings` table. Anything still sitting in localStorage
 * from earlier builds is imported once and then removed.
 */
export async function initSettings(): Promise<void> {
  const stored = await readStored();

  // Unreadable is not empty. Showing "welcome, choose a folder and a key" over an existing
  // installation hides the real problem and invites the user to redo work that is not lost.
  // It also must not persist defaults: that would overwrite what is actually stored.
  if (stored === null) {
    settingsUnreadable.value = true;
    firstRun.value = false;
    return;
  }

  if (Object.keys(stored).length === 0) {
    const legacy = loadJson<Record<string, unknown> | null>(LEGACY_KEY, null);
    if (legacy && typeof legacy === "object") {
      apply(legacy);
      settings.permission = permissionFromLegacy(legacy);
      settings.onboarded = true;
      firstRun.value = false;
      try {
        localStorage.removeItem(LEGACY_KEY);
      } catch {
        // storage unavailable — the import still happened
      }
      loaded = true;
      watch(settings, schedulePersist, { deep: true });
      await persistNow();
      return;
    }
  }

  const decoded: Record<string, unknown> = {};
  for (const [key, raw] of Object.entries(stored)) {
    try {
      decoded[key] = JSON.parse(raw);
    } catch {
      // skip unreadable values rather than breaking startup
    }
  }
  apply(decoded);
  if (!("permission" in decoded)) settings.permission = permissionFromLegacy(decoded);

  // An untouched list moves forward with the defaults. Recognising it by exact equality makes
  // this self-limiting: as soon as anybody adds or removes a program the lists stop matching,
  // and nothing is ever re-added behind their back.
  // Each release adds a snapshot of the list it shipped, so an installation that skipped one
  // release still moves forward; a customised list matches none of them and is left alone.
  const migrated =
    sameCommands(settings.allowedCommands, PREVIOUS_DEFAULT_COMMANDS) ||
    sameCommands(settings.allowedCommands, PREVIOUS_TEXT_TOOL_COMMANDS);
  if (migrated) settings.allowedCommands = [...defaultAllowedCommands];

  // A database with no settings rows is a fresh install. Existing installations — including
  // ones from before this flag existed — never see the first-start dialog.
  firstRun.value = Object.keys(stored).length === 0;
  if (!firstRun.value) settings.onboarded = true;

  // One line per start, and the only way to tell "the dialog thinks you are new" apart from
  // "the table is really empty" on a machine I cannot look at. A report of "the dialog comes
  // back every start" needs this number next to it.
  void invoke("log_message", {
    text:
      `settings: read ${Object.keys(stored).length} keys, firstRun=${firstRun.value}, ` +
      `appearance=${settings.appearance}, fontSize=${settings.fontSize}`,
  }).catch(() => undefined);

  loaded = true;
  watch(settings, schedulePersist, { deep: true });
  watchModeModel();

  // The migration above runs before the watcher exists, so nothing would write it out until the
  // next unrelated change or the next time the window is hidden. Writing it now costs one line
  // once: on the following start the stored list no longer matches the old one.
  if (migrated) await persistNow();
}

/**
 * Switching the mode moves the pre-selected model with it — but only while the current model
 * is still the default of the mode being left. A model the user chose by hand stays.
 */
function watchModeModel(): void {
  watch(
    () => settings.mode,
    (mode, previous) => {
      if (previous && settings.model === defaultModelFor(previous)) {
        settings.model = defaultModelFor(mode);
      }
    },
  );
}

const media =
  typeof window === "undefined" ? null : window.matchMedia("(prefers-color-scheme: dark)");

/**
 * The OS preference, held reactively.
 *
 * Reading `media.matches` directly inside the computed below looked right and was wrong: a
 * computed caches on its *reactive* dependencies, and `matches` is a plain property. So in
 * System mode the first answer was kept forever, the `change` listener re-applied that stale
 * value, and switching the OS theme did nothing until the appearance was picked again.
 */
const prefersDark = ref(media?.matches ?? true);

export const resolvedTheme = computed<"light" | "dark">(() =>
  settings.appearance === "system" ? (prefersDark.value ? "dark" : "light") : settings.appearance,
);

function applyTheme(): void {
  if (typeof document !== "undefined") {
    document.documentElement.dataset.theme = resolvedTheme.value;
  }
}

export function initTheme(): void {
  applyTheme();
  watch(resolvedTheme, applyTheme);
  // Assigning the ref is what makes the computed above invalidate; the mutation itself is the
  // signal, so `applyTheme` must not be called directly here.
  media?.addEventListener("change", (event) => {
    prefersDark.value = event.matches;
  });
}

/**
 * Reading size for the conversation and the previews. Applied as a CSS variable so a
 * single value reaches every place that shows prose.
 */
export function initReadingSize(): void {
  const apply = () => {
    document.documentElement.style.setProperty("--reading-font-size", `${settings.fontSize}px`);
  };
  apply();
  watch(() => settings.fontSize, apply);
}

export function update<K extends keyof SettingsState>(key: K, value: SettingsState[K]): void {
  settings[key] = value;
}

/**
 * A window that loses focus may be closed a moment later. The debounce is right while the user
 * types; it is wrong as the last thing that happens before the process ends. Hiding is the common
 * case (the tray keeps the app alive), quitting is the one that loses data.
 */
if (typeof window !== "undefined") {
  window.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "hidden") void flushSettings();
  });
  window.addEventListener("pagehide", () => void flushSettings());
}
