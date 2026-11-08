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
export const defaultAllowedCommands = [
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

async function persistNow(): Promise<void> {
  for (const key of PERSISTED_KEYS) {
    try {
      await invoke("settings_set", { key, value: JSON.stringify(settings[key]) });
    } catch {
      // persistence is best effort; the app must keep working without it
    }
  }
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
 * Settings live in the SQLite `settings` table. Anything still sitting in localStorage
 * from earlier builds is imported once and then removed.
 */
export async function initSettings(): Promise<void> {
  let stored: Record<string, string> = {};
  try {
    stored = await invoke<Record<string, string>>("settings_all");
  } catch {
    stored = {};
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

  // A database with no settings rows is a fresh install. Existing installations — including
  // ones from before this flag existed — never see the first-start dialog.
  firstRun.value = Object.keys(stored).length === 0;
  if (!firstRun.value) settings.onboarded = true;

  loaded = true;
  watch(settings, schedulePersist, { deep: true });
  watchModeModel();
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

function prefersDark(): boolean {
  return media?.matches ?? true;
}

export const resolvedTheme = computed<"light" | "dark">(() =>
  settings.appearance === "system" ? (prefersDark() ? "dark" : "light") : settings.appearance,
);

function applyTheme(): void {
  if (typeof document !== "undefined") {
    document.documentElement.dataset.theme = resolvedTheme.value;
  }
}

export function initTheme(): void {
  applyTheme();
  watch(resolvedTheme, applyTheme);
  media?.addEventListener("change", applyTheme);
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
