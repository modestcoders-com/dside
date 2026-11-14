import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { FileNode } from "../types";
import { t } from "../i18n";
import { settings } from "../stores/settings";

export const tree = ref<FileNode[]>([]);
export const treeLoading = ref(false);
export const treeError = ref<string | null>(null);

export async function reloadTree(): Promise<void> {
  if (!settings.workspace) {
    tree.value = [];
    treeError.value = t("workspace.none");
    return;
  }
  treeLoading.value = true;
  treeError.value = null;
  try {
    // Only the top level: everything deeper is fetched when a folder is expanded, so
    // opening a workspace with thousands of files stays instant.
    tree.value = await invoke<FileNode[]>("read_workspace_tree", {
      root: settings.workspace,
      depth: 0,
    });
  } catch (error) {
    const message = String(error);
    // The backend names the missing path. Show the user the same fact in their language
    // instead of an empty tree that looks like a workspace with no files.
    treeError.value =
      message.includes("does not exist") || message.includes("not a directory")
        ? t("workspace.missing", { path: settings.workspace })
        : message;
    tree.value = [];
  } finally {
    treeLoading.value = false;
  }
}

/** Reads one directory level into a folder node. */
export async function loadChildren(node: FileNode): Promise<void> {
  try {
    node.children = await invoke<FileNode[]>("read_dir_children", { path: node.path });
  } catch {
    node.children = [];
  }
  node.loaded = true;
}

/** True for a path that names a place: `/…` on Unix, `C:\…` or a UNC path on Windows. */
export function isAbsolutePath(path: string): boolean {
  const trimmed = path.trim();
  return trimmed.startsWith("/") || /^[A-Za-z]:[\\/]/.test(trimmed) || trimmed.startsWith("\\\\");
}

export async function ensureWorkspace(): Promise<void> {
  // A stored "." is a leftover: the default was `HOME` with a dot as fallback, and Windows has
  // no `HOME`. Every fresh install there began with a dot that names no folder, so the field
  // looked set while the file tree had nothing to show. Healing it here fixes an existing
  // installation without asking the user to reinstall.
  if (settings.workspace && isAbsolutePath(settings.workspace)) return;
  try {
    settings.workspace = await invoke<string>("default_workspace");
  } catch {
    settings.workspace = "";
  }
}

export async function initWorkspace(): Promise<void> {
  await ensureWorkspace();
  await reloadTree();
}

export async function readFile(path: string): Promise<string> {
  return invoke<string>("read_text_file", { path });
}
