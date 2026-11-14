<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import Icon from "./Icon.vue";
import DeepSeekWhale from "./DeepSeekWhale.vue";
import WorkspacePicker from "./WorkspacePicker.vue";
import { deepSeekMark } from "./deepseek-mark";
import { t } from "../i18n";
import { engineReady, saveApiKey } from "../composables/useEngine";
import { ensureWorkspace, reloadTree } from "../composables/useWorkspace";
import { firstRun, flushSettings, settings } from "../stores/settings";
import type { AppMode } from "../types";

const selected = ref<AppMode>(settings.mode);
const workspaceDraft = ref(settings.workspace);
const workspacePickerOpen = ref(false);
const keyDraft = ref("");
const saving = ref(false);
const saved = ref(false);
const starting = ref(false);
const error = ref<string | null>(null);

onMounted(async () => {
  await ensureWorkspace();
  workspaceDraft.value = settings.workspace;
});

async function saveKey(): Promise<void> {
  const value = keyDraft.value.trim();
  if (!value || saving.value) return;
  saving.value = true;
  error.value = null;
  try {
    await saveApiKey(value);
    keyDraft.value = "";
    saved.value = true;
  } catch (cause) {
    error.value = String(cause);
  } finally {
    saving.value = false;
  }
}

/** The mode is the one decision that changes the whole first impression; it persists. */
async function start(): Promise<void> {
  const workspace = workspaceDraft.value.trim();
  if (!workspace) {
    error.value = t("firstRun.workspaceHint");
    return;
  }
  starting.value = true;
  error.value = null;
  try {
    // The same command the file tree uses: an invalid folder fails here, not in the tree.
    await invoke("read_workspace_tree", { root: workspace, depth: 0 });
    settings.workspace = workspace;
    settings.mode = selected.value;
    settings.onboarded = true;
    firstRun.value = false;
    // Written before the dialog goes away: what this decides is exactly what the *next* start
    // reads, and a debounced write can be lost if the app is closed a moment later.
    await flushSettings();
    void reloadTree();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    starting.value = false;
  }
}

async function later(): Promise<void> {
  settings.onboarded = true;
  firstRun.value = false;
  await flushSettings();
}
</script>

<template>
  <div class="backdrop">
    <div class="dialog" role="dialog" aria-modal="true">
      <div class="brand">
        <DeepSeekWhale class="whale" :height="36" />
        <span class="mark" v-html="deepSeekMark"></span>
      </div>

      <h2>{{ t("firstRun.title") }}</h2>
      <p class="hint">{{ t("firstRun.hint") }}</p>

      <div class="modes">
        <button
          class="mode"
          :class="{ active: selected === 'coding' }"
          type="button"
          @click="selected = 'coding'"
        >
          <Icon name="grid" :size="18" />
          <span class="mode-title">{{ t("settings.general.modeCodingTitle") }}</span>
          <span class="mode-desc">{{ t("settings.general.modeCodingDesc") }}</span>
        </button>
        <button
          class="mode"
          :class="{ active: selected === 'everyday' }"
          type="button"
          @click="selected = 'everyday'"
        >
          <Icon name="monitor" :size="18" />
          <span class="mode-title">{{ t("settings.general.modeEverydayTitle") }}</span>
          <span class="mode-desc">{{ t("settings.general.modeEverydayDesc") }}</span>
        </button>
      </div>

      <div class="keys">
        <label class="label" for="first-run-workspace">{{ t("viewFile.workspace") }}</label>
        <div class="workspace-row">
          <input
            id="first-run-workspace"
            v-model="workspaceDraft"
            class="input"
            spellcheck="false"
          />
          <button
            class="pick"
            type="button"
            :title="t('workspacePicker.chooseShort')"
            @click="workspacePickerOpen = true"
          >
            <Icon name="folder" :size="15" />
          </button>
        </div>
        <p class="hint small">{{ t("firstRun.workspaceHint") }}</p>
      </div>

      <div class="keys">
        <label class="label" for="first-run-key">{{ t("model.apiKey") }}</label>
        <div class="secret">
          <input
            id="first-run-key"
            v-model="keyDraft"
            class="input"
            type="password"
            autocomplete="off"
            :placeholder="engineReady ? '••••••••' : ''"
            @keydown.enter.prevent="saveKey"
          />
          <button
            class="mini"
            type="button"
            :disabled="saving || !keyDraft.trim()"
            @click="saveKey"
          >
            <Icon :name="saved ? 'check' : 'arrow-right'" :size="14" />
          </button>
        </div>
        <p class="hint small">{{ t("firstRun.keyHint") }}</p>
        <p v-if="engineReady" class="ok">{{ t("model.storedKeyring") }}</p>
        <p v-if="error" class="error">{{ error }}</p>
      </div>

      <div class="actions">
        <button class="later" type="button" @click="later">{{ t("firstRun.later") }}</button>
        <button class="start" type="button" :disabled="starting" @click="start">
          {{ starting ? t("viewFile.loading") : t("firstRun.start") }}
        </button>
      </div>
    </div>
    <WorkspacePicker
      :open="workspacePickerOpen"
      :initial="workspaceDraft"
      @choose="workspaceDraft = $event"
      @close="workspacePickerOpen = false"
    />
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 90;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: rgb(0 0 0 / 55%);
  backdrop-filter: blur(2px);
}

.dialog {
  width: min(560px, 100%);
  max-height: calc(100vh - 48px);
  overflow: auto;
  padding: 24px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  background: var(--bg-app);
  box-shadow: 0 24px 70px rgb(0 0 0 / 55%);
}

.brand {
  display: flex;
  align-items: center;
  gap: 10px;
}

.whale {
  height: 36px;
  width: auto;
}

.mark :deep(svg) {
  display: block;
  height: 18px;
  width: auto;
  color: var(--text-strong);
}

h2 {
  margin: 14px 0 4px;
  color: var(--text-strong);
  font-size: 19px;
}

.hint {
  margin: 0;
  color: var(--text-muted);
  font-size: 12.5px;
  line-height: 1.55;
}

.hint.small {
  margin-top: 6px;
  font-size: 11px;
}

.modes {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  margin: 16px 0;
}

@media (max-width: 520px) {
  .modes {
    grid-template-columns: 1fr;
  }
}

.mode {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 6px;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-panel);
  color: var(--text);
  text-align: left;
}

.mode:hover {
  background: var(--bg-hover);
}

.mode.active {
  border-color: var(--accent);
  background: var(--bg-active);
}

.mode-title {
  color: var(--text-strong);
  font-size: 13px;
  font-weight: 600;
}

.mode-desc {
  color: var(--text-muted);
  font-size: 11px;
  line-height: 1.4;
}

.keys {
  margin: 0 0 16px;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-panel);
}

.label {
  color: var(--text-muted);
  font-size: 12px;
}

.secret {
  display: flex;
  gap: 6px;
  margin-top: 6px;
}

.input {
  flex: 1;
  min-width: 0;
  height: 30px;
  padding: 0 9px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text-strong);
  font-size: 12px;
}

.input.full {
  display: block;
  width: 100%;
  margin-top: 6px;
  font-family: var(--mono);
  font-size: 11.5px;
}

.mini {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-elevated);
  color: var(--text-strong);
}

.mini:hover:not(:disabled) {
  background: var(--bg-hover);
}

.mini:disabled {
  opacity: 0.5;
}

.ok {
  margin: 6px 0 0;
  color: var(--accent);
  font-size: 11px;
}

.error {
  margin: 6px 0 0;
  color: var(--danger);
  font-size: 11px;
}

.actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.later,
.start {
  height: 30px;
  padding: 0 14px;
  border-radius: var(--radius-sm);
  font-size: 12px;
}

.later {
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-muted);
}

.later:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.start {
  border: 1px solid var(--accent);
  background: var(--accent);
  color: var(--accent-contrast);
  font-weight: 600;
}

.start:hover {
  filter: brightness(1.08);
}

.workspace-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.workspace-row .input {
  flex: 1;
  min-width: 0;
}

.pick {
  display: grid;
  place-items: center;
  width: 36px;
  height: 36px;
  flex: none;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-muted);
}

.pick:hover {
  background: var(--bg-hover);
  color: var(--text-strong);
}
</style>
