import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { settings } from "../stores/settings";
import { activeSessionId, sessions } from "../stores/sessions";
import { reloadTree } from "./useWorkspace";
import { createSession, openSession } from "./useEngine";
import type { ProjectFolder } from "../lib/projects";
import { isDirectChild, projectLabel, samePath } from "../lib/projects";

/**
 * The folders of the project base, and switching the active one.
 *
 * Two values, deliberately: `projectsRoot` is only ever listed (the dropdown's source), and
 * `settings.workspace` stays what the tools read and write in. Pointing `workspace` at the
 * base would hand write access to every project at once.
 */
export const projectFolders = ref<ProjectFolder[]>([]);
export const projectError = ref<string | null>(null);
export const projectsLoading = ref(false);

export async function loadProjectFolders(): Promise<void> {
  const root = settings.projectsRoot.trim();
  projectError.value = null;
  if (!root) {
    projectFolders.value = [];
    return;
  }
  projectsLoading.value = true;
  try {
    projectFolders.value = await invoke<ProjectFolder[]>("list_project_folders", { root });
  } catch (error) {
    projectFolders.value = [];
    projectError.value = String(error);
  } finally {
    projectsLoading.value = false;
  }
}

/** The project the tools are pointing at, as it appears in the rail. */
export const activeProject = computed(() => {
  const workspace = settings.workspace.trim();
  if (!workspace) return "";
  return projectLabel(workspace);
});

/**
 * Makes `path` the active project. The file tree is reloaded here and not by the caller: a
 * project switch that leaves the old tree on screen is the one state nobody can read.
 */
export async function activateProject(path: string): Promise<void> {
  const next = path.trim();
  if (next === settings.workspace) return;
  settings.workspace = next;
  await reloadTree();
}

/**
 * Re-binds the open thread to `path`, so the conversation and the folder the tools work in
 * never disagree. Without this the rail would show a thread under projA while the next
 * message wrote into projB — and a restart would silently undo whichever one lost.
 */
export async function bindActiveThread(path: string): Promise<void> {
  const id = activeSessionId.value;
  const thread = sessions.value.find((entry) => entry.id === id);
  if (thread && !samePath(thread.workspace || "", path)) {
    await invoke("session_set_workspace", { id, workspace: path });
    thread.workspace = path;
  }
  await activateProject(path);
}

/**
 * Choosing a project in the rail means working in that project: the newest thread of it
 * opens, or a new one is started. Keeping the old conversation on screen would show text
 * that belongs to a folder the tools have just left.
 */
export async function switchProject(path: string): Promise<void> {
  await activateProject(path);
  const threads = sessions.value.filter(
    (entry) => entry.group === "project" && samePath(entry.workspace || "", path.trim()),
  );
  // `sessions` arrives newest first, so the first match is the most recent thread.
  if (threads.length) await openSession(threads[0].id);
  else await createSession();
}

/** True when the active project is one of the base's folders — used to mark it in the list. */
export function isKnownProject(path: string): boolean {
  if (!path) return false;
  return (
    projectFolders.value.some((folder) => folder.path === path) ||
    isDirectChild(settings.projectsRoot, path)
  );
}
