<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import Icon from "./Icon.vue";
import { t } from "../i18n";
import { isAbsolutePath } from "../composables/useWorkspace";

/** A folder browser on top of the same `read_dir_children` command the file tree uses. */
const props = defineProps<{ open: boolean; initial?: string }>();
const emit = defineEmits<{ choose: [path: string]; close: [] }>();

interface Entry {
  name: string;
  path: string;
  isFolder: boolean;
}

const current = ref("");
const pathDraft = ref("");
const entries = ref<Entry[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const input = ref<HTMLInputElement | null>(null);

function parentOf(path: string): string {
  const normalized = path.replace(/[\\/]+$/, "");
  if (/^[A-Za-z]:$/.test(normalized)) return `${normalized}\\`;
  if (normalized === "") return path;
  const index = Math.max(normalized.lastIndexOf("/"), normalized.lastIndexOf("\\"));
  if (index < 0) return normalized;
  if (index === 0) return "/";
  const parent = normalized.slice(0, index);
  return /^[A-Za-z]:$/.test(parent) ? `${parent}\\` : parent;
}

function isRoot(path: string): boolean {
  return path === "/" || /^[A-Za-z]:[\\/]$/.test(path);
}

async function load(path: string): Promise<boolean> {
  if (!path || loading.value) return false;
  loading.value = true;
  error.value = null;
  try {
    const nodes = await invoke<Entry[]>("read_dir_children", { path });
    current.value = path;
    pathDraft.value = path;
    entries.value = nodes
      .filter((node) => node.isFolder)
      .sort((a, b) => a.name.localeCompare(b.name));
    return true;
  } catch (cause) {
    error.value = String(cause);
    return false;
  } finally {
    loading.value = false;
  }
}

async function start(): Promise<void> {
  error.value = null;
  const initial = props.initial?.trim();
  // A relative path — a dot left over from the old default — names no folder to open: the
  // listing came back empty and there was nothing to choose, which is what a Windows install
  // showed. Skip it and start at home instead.
  if (initial && isAbsolutePath(initial) && (await load(initial))) {
    void nextTick(() => input.value?.focus());
    return;
  }
  try {
    const home = await invoke<string>("default_workspace");
    await load(home);
  } catch (cause) {
    error.value = String(cause);
  }
  void nextTick(() => input.value?.focus());
}

watch(
  () => props.open,
  (open) => {
    if (open) void start();
  },
);

function onKey(event: KeyboardEvent): void {
  if (!props.open || event.key !== "Escape") return;
  event.preventDefault();
  event.stopPropagation();
  emit("close");
}

onMounted(() => window.addEventListener("keydown", onKey, true));
onBeforeUnmount(() => window.removeEventListener("keydown", onKey, true));

function go(): void {
  const path = pathDraft.value.trim();
  // "Öffnen" used to take anything, and a relative path listed whatever the process happened
  // to sit in — or nothing. Only a full path names a place.
  if (!isAbsolutePath(path)) {
    error.value = t("workspacePicker.absoluteOnly");
    return;
  }
  error.value = null;
  void load(path);
}

function up(): void {
  if (!current.value || isRoot(current.value)) return;
  void load(parentOf(current.value));
}

function choose(): void {
  if (!current.value) return;
  emit("choose", current.value);
  emit("close");
}
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="picker-backdrop" @click.self="emit('close')">
      <div class="picker" role="dialog" aria-modal="true" :aria-label="t('workspacePicker.title')">
        <h2>{{ t("workspacePicker.title") }}</h2>
        <p class="hint">{{ t("workspacePicker.hint") }}</p>

        <div class="path-row">
          <button class="mini" type="button" :title="t('workspacePicker.up')" @click="up">
            <Icon name="arrow-left" :size="14" />
          </button>
          <input
            ref="input"
            v-model="pathDraft"
            class="path-input"
            spellcheck="false"
            @keydown.enter.prevent="go"
          />
          <button class="mini-text" type="button" @click="go">
            {{ t("workspacePicker.go") }}
          </button>
        </div>

        <p v-if="error" class="error">{{ error }}</p>

        <div class="folder-list">
          <p v-if="loading" class="note">{{ t("workspacePicker.loading") }}</p>
          <p v-else-if="entries.length === 0" class="note">{{ t("workspacePicker.empty") }}</p>
          <button
            v-for="entry in entries"
            :key="entry.path"
            class="folder"
            type="button"
            @click="load(entry.path)"
          >
            <Icon name="folder" :size="14" />
            <span>{{ entry.name }}</span>
          </button>
        </div>

        <div class="actions">
          <button class="mini-text" type="button" @click="emit('close')">
            {{ t("workspacePicker.cancel") }}
          </button>
          <button class="choose" type="button" :disabled="!current || loading" @click="choose">
            {{ t("workspacePicker.choose") }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.picker-backdrop {
  position: fixed;
  inset: 0;
  z-index: 3200;
  display: grid;
  place-items: center;
  padding: 16px;
  background: rgba(0, 0, 0, 0.62);
}

.picker {
  display: flex;
  flex-direction: column;
  width: min(560px, 100%);
  max-height: min(640px, calc(100vh - 32px));
  padding: 18px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-lg);
  background: var(--bg-elevated);
  box-shadow: 0 24px 70px rgba(0, 0, 0, 0.55);
  color: var(--text);
}

h2 {
  margin: 0 0 4px;
  color: var(--text-strong);
  font-size: 16px;
  font-weight: 600;
}

.hint {
  margin: 0 0 12px;
  color: var(--text-muted);
  font-size: 12px;
}

.path-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.path-input {
  flex: 1;
  min-width: 0;
  height: 30px;
  padding: 0 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text);
  font-family: var(--mono);
  font-size: 11.5px;
  outline: none;
}

.path-input:focus {
  border-color: var(--accent);
}

.mini {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  flex: none;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-muted);
}

.mini:hover {
  background: var(--bg-hover);
  color: var(--text-strong);
}

.folder-list {
  flex: 1;
  min-height: 160px;
  margin-top: 10px;
  padding: 4px;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-panel);
}

.folder {
  display: flex;
  align-items: center;
  gap: 7px;
  width: 100%;
  padding: 6px 8px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text);
  font-size: 12px;
  text-align: left;
}

.folder:hover {
  background: var(--bg-hover);
}

.note {
  margin: 12px;
  color: var(--text-muted);
  font-size: 12px;
}

.error {
  margin: 8px 0 0;
  color: var(--danger);
  font-size: 12px;
}

.actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 14px;
}

.choose {
  height: 30px;
  padding: 0 12px;
  border: 1px solid var(--accent);
  border-radius: var(--radius-sm);
  background: var(--accent);
  color: var(--bg-app);
  font-size: 12px;
  font-weight: 600;
}

.choose:disabled {
  cursor: default;
  opacity: 0.45;
}
</style>
