/**
 * Project helpers. A "project" is a folder under the project base; a thread belongs to at
 * most one. Nothing here guesses: a thread whose folder is gone still gets a group, so it
 * does not silently disappear from the rail.
 */

export interface ProjectFolder {
  name: string;
  path: string;
}

/** The last segment of a path — how a project is called in the rail. */
export function projectLabel(path: string): string {
  return path.replace(/[\\/]+$/, "").split(/[\\/]/).pop() || path;
}

/** Compares two folders the way the file system might spell them (trailing slashes). */
export function samePath(a: string, b: string): boolean {
  const normalize = (value: string) => value.replace(/[\\/]+$/, "");
  return normalize(a) === normalize(b);
}

/** True when `path` is a direct child of `root`. */
export function isDirectChild(root: string, path: string): boolean {
  if (!root || !path) return false;
  const trim = (value: string) => value.replace(/[\\/]+$/, "");
  const parent = trim(root);
  const child = trim(path);
  const rest = child.startsWith(`${parent}/`) || child.startsWith(`${parent}\\`)
    ? child.slice(parent.length + 1)
    : "";
  return rest.length > 0 && !/[\\/]/.test(rest);
}

export interface ProjectGroup<T> {
  /** Empty for the "no project" group. */
  workspace: string;
  label: string;
  threads: T[];
}

/**
 * Threads grouped by their project, in the order the groups first appear. Threads without a
 * workspace collect in one group at the end — the same place the dropdown offers them.
 */
export function groupByProject<T extends { id: string; workspace: string }>(
  threads: T[],
  emptyLabel: string,
): ProjectGroup<T>[] {
  const groups = new Map<string, ProjectGroup<T>>();
  const loose: ProjectGroup<T> = { workspace: "", label: emptyLabel, threads: [] };

  for (const thread of threads) {
    if (!thread.workspace.trim()) {
      loose.threads.push(thread);
      continue;
    }
    const key = thread.workspace.replace(/[\\/]+$/, "");
    const group = groups.get(key) ?? {
      workspace: key,
      label: projectLabel(key),
      threads: [],
    };
    group.threads.push(thread);
    groups.set(key, group);
  }

  const ordered = [...groups.values()];
  if (loose.threads.length) ordered.push(loose);
  return ordered;
}
