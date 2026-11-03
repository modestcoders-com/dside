<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { t } from "../i18n";
import { resolvedTheme, settings } from "../stores/settings";

/**
 * The shell lives in Rust (portable-pty); this component is only the screen. The terminal
 * stays mounted while its work-area tab exists, so switching to the file tab does not kill
 * the shell. Closing the tab unmounts it and the backend stops the child.
 */
const props = defineProps<{ active?: boolean }>();

const host = ref<HTMLDivElement | null>(null);
const exited = ref(false);
const failure = ref<string | null>(null);

const ID = "terminal";

let term: Terminal | null = null;
let fit: FitAddon | null = null;
let observer: ResizeObserver | null = null;
let unlistenOutput: UnlistenFn | null = null;
let unlistenExit: UnlistenFn | null = null;

const theme = computed(() => {
  const dark = resolvedTheme.value === "dark";
  return {
    background: dark ? "#1e1e1e" : "#ffffff",
    foreground: dark ? "#d4d4d4" : "#1f2328",
    cursor: dark ? "#aeafad" : "#1f2328",
    selectionBackground: dark ? "#264f78" : "#add6ff",
  };
});

function fitNow(): void {
  if (!fit || !host.value) return;
  if (host.value.clientWidth === 0 || host.value.clientHeight === 0) return;
  try {
    fit.fit();
  } catch {
    // the panel can be between two layouts; the next resize will fit again
  }
}

function resizePty(): void {
  fitNow();
  if (!term) return;
  void invoke("pty_resize", { id: ID, cols: term.cols, rows: term.rows }).catch(() => undefined);
}

/**
 * The folder this shell was started in. A project switch does not move a running process, and
 * a shell that silently claims to be somewhere else would be worse than one that says where
 * it is — so the bar shows the start folder, not the current one.
 */
const startCwd = ref("");

async function openPty(): Promise<void> {
  if (!term) return;
  exited.value = false;
  failure.value = null;
  startCwd.value = settings.workspace;
  try {
    await invoke("pty_open", {
      id: ID,
      cwd: settings.workspace,
      cols: term.cols,
      rows: term.rows,
    });
    resizePty();
  } catch (error) {
    failure.value = String(error);
  }
}

function clearTerminal(): void {
  term?.clear();
}

async function restart(): Promise<void> {
  await invoke("pty_close", { id: ID }).catch(() => undefined);
  term?.reset();
  await openPty();
}

onMounted(async () => {
  if (!host.value) return;

  const instance = new Terminal({
    cursorBlink: true,
    fontSize: 13,
    fontFamily: "Menlo, 'DejaVu Sans Mono', Consolas, monospace",
    scrollback: 5000,
    theme: theme.value,
  });
  const fitAddon = new FitAddon();

  instance.open(host.value);
  instance.loadAddon(fitAddon);
  fitNow();

  instance.onData((data) => {
    void invoke("pty_write", { id: ID, data }).catch(() => undefined);
  });
  instance.onResize(({ cols, rows }) => {
    void invoke("pty_resize", { id: ID, cols, rows }).catch(() => undefined);
  });

  unlistenOutput = await listen<{ id: string; data: string }>("pty-output", (event) => {
    if (event.payload.id === ID) instance.write(event.payload.data);
  });
  unlistenExit = await listen<{ id: string }>("pty-exit", (event) => {
    if (event.payload.id !== ID) return;
    exited.value = true;
    instance.write("\r\n\x1b[2m[" + t("terminal.exited") + "]\x1b[0m\r\n");
  });

  observer = new ResizeObserver(() => resizePty());
  observer.observe(host.value);

  term = instance;
  fit = fitAddon;
  await openPty();

  if (props.active) {
    fitNow();
    instance.focus();
  }
});

watch(
  () => props.active,
  (active) => {
    if (!active) return;
    void nextTick(() => {
      fitNow();
      term?.focus();
    });
  },
);

watch(resolvedTheme, () => {
  if (term) term.options.theme = theme.value;
});

onBeforeUnmount(() => {
  unlistenOutput?.();
  unlistenExit?.();
  observer?.disconnect();
  observer = null;
  void invoke("pty_close", { id: ID }).catch(() => undefined);
  term?.dispose();
  term = null;
  fit = null;
});
</script>

<template>
  <div class="terminal-panel">
    <div class="terminal-bar">
      <span class="terminal-title">{{ t("terminal.title") }}</span>
      <span v-if="startCwd" class="terminal-cwd" :title="startCwd">{{ startCwd }}</span>
      <span v-if="exited" class="terminal-note">{{ t("terminal.exited") }}</span>
      <span v-if="failure" class="terminal-note error">{{ t("terminal.failed", { error: failure }) }}</span>
      <span class="spacer"></span>
      <button class="mini-text" type="button" @click="clearTerminal">
        {{ t("terminal.clear") }}
      </button>
      <button class="mini-text" type="button" @click="restart">
        {{ t("terminal.restart") }}
      </button>
    </div>
    <div ref="host" class="terminal-host" data-terminal-input></div>
  </div>
</template>

<style scoped>
.terminal-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  background: var(--bg-panel);
}

.terminal-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 32px;
  padding: 0 8px;
  border-bottom: 1px solid var(--border);
  color: var(--text-muted);
  font-size: 11px;
}

.terminal-title {
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.terminal-cwd {
  max-width: 45%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--mono);
  font-size: 10.5px;
  opacity: 0.75;
}

.terminal-note {
  overflow: hidden;
  color: var(--text-muted);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.terminal-note.error {
  color: var(--danger);
}

.spacer {
  flex: 1;
}

.terminal-host {
  flex: 1;
  min-height: 0;
  padding: 4px 6px 6px;
  overflow: hidden;
}

.terminal-host :deep(.xterm) {
  height: 100%;
}
</style>
