<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import Icon from "../Icon.vue";
import { t } from "../../i18n";
import { settings } from "../../stores/settings";

interface GitEntry {
  path: string;
  code: string;
  staged: boolean;
  untracked: boolean;
  conflicted: boolean;
}

interface GitStatus {
  branch: string;
  head: string;
  entries: GitEntry[];
  /** How many entries git reported before the cap. */
  total: number;
  /** True when the list was cut — a repository this large is not a panel. */
  truncated: boolean;
}

interface DiffLine {
  text: string;
  kind: string;
}

const status = ref<GitStatus | null>(null);
const branches = ref<string[]>([]);
const branch = ref("");
const loading = ref(false);
const busy = ref(false);
const error = ref<string | null>(null);
const notice = ref<string | null>(null);
const message = ref("");
const selected = ref<GitEntry | null>(null);
const diff = ref("");
const diffLoading = ref(false);

const staged = computed(() => status.value?.entries.filter((entry) => entry.staged) ?? []);
const unstaged = computed(
  () => status.value?.entries.filter((entry) => !entry.staged && !entry.untracked) ?? [],
);
const untracked = computed(() => status.value?.entries.filter((entry) => entry.untracked) ?? []);
const hasStaged = computed(() => staged.value.length > 0);

const errorText = computed(() => {
  if (!error.value) return "";
  // The backend answers with a marker, not a sentence: no repository for this project, no
  // matter whether there is none at all or only a parent's. One calm line in the user's language.
  return error.value.toLowerCase().includes("no_git_repository")
    ? t("git.noRepo")
    : error.value;
});

const diffLines = computed<DiffLine[]>(() =>
  diff.value.split("\n").map((text) => ({ text, kind: lineKind(text) })),
);

function lineKind(line: string): string {
  if (line.startsWith("@@")) return "hunk";
  if (
    line.startsWith("diff ") ||
    line.startsWith("index ") ||
    line.startsWith("--- ") ||
    line.startsWith("+++ ")
  ) {
    return "meta";
  }
  if (line.startsWith("+")) return "add";
  if (line.startsWith("-")) return "del";
  return "";
}

async function loadDiff(entry: GitEntry): Promise<void> {
  selected.value = entry;
  diff.value = "";
  diffLoading.value = true;
  try {
    diff.value = await invoke<string>("git_panel_diff", {
      workspace: settings.workspace,
      path: entry.path,
      staged: entry.staged,
      untracked: entry.untracked,
    });
  } catch (cause) {
    diff.value = String(cause);
  } finally {
    diffLoading.value = false;
  }
}

async function refresh(): Promise<void> {
  if (loading.value) return;
  loading.value = true;
  error.value = null;
  const previous = selected.value;
  try {
    const next = await invoke<GitStatus>("git_panel_status", {
      workspace: settings.workspace,
    });
    status.value = next;
    branch.value = next.branch;
    try {
      branches.value = await invoke<string[]>("git_panel_branches", {
        workspace: settings.workspace,
      });
    } catch {
      branches.value = [];
    }
    if (previous) {
      const match = next.entries.find(
        (entry) => entry.path === previous.path && entry.staged === previous.staged,
      );
      if (match) await loadDiff(match);
      else {
        selected.value = null;
        diff.value = "";
      }
    }
  } catch (cause) {
    status.value = null;
    branches.value = [];
    branch.value = "";
    selected.value = null;
    diff.value = "";
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}

async function checkoutBranch(): Promise<void> {
  const target = branch.value;
  const current = status.value?.branch ?? "";
  if (!target || target === current || busy.value) return;
  busy.value = true;
  error.value = null;
  notice.value = null;
  try {
    await invoke("git_panel_checkout", {
      workspace: settings.workspace,
      branch: target,
    });
    await refresh();
  } catch (cause) {
    error.value = String(cause);
    branch.value = current;
  } finally {
    busy.value = false;
  }
}

async function pull(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  error.value = null;
  notice.value = null;
  try {
    const output = await invoke<string>("git_panel_pull", {
      workspace: settings.workspace,
    });
    notice.value = output || t("git.pullDone");
    await refresh();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    busy.value = false;
  }
}

async function toggleStage(entry: GitEntry): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  error.value = null;
  notice.value = null;
  try {
    await invoke(entry.staged ? "git_panel_unstage" : "git_panel_stage", {
      workspace: settings.workspace,
      paths: [entry.path],
    });
    await refresh();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    busy.value = false;
  }
}

async function stageAll(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  error.value = null;
  notice.value = null;
  try {
    await invoke("git_panel_stage_all", { workspace: settings.workspace });
    await refresh();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    busy.value = false;
  }
}

async function commit(): Promise<void> {
  const text = message.value.trim();
  if (!text || busy.value || !hasStaged.value) return;
  busy.value = true;
  error.value = null;
  notice.value = null;
  try {
    notice.value =
      (await invoke<string>("git_panel_commit", {
        workspace: settings.workspace,
        message: text,
      })) || t("git.committed");
    message.value = "";
    await refresh();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    busy.value = false;
  }
}

function groupTitle(list: GitEntry[]): string {
  return list.length > 0 ? String(list.length) : "";
}

onMounted(() => {
  void refresh();
});

// A project switch changes which repository this panel is about. Without this the list keeps
// showing the previous project's files — the panel is mounted once and lives on.
watch(
  () => settings.workspace,
  () => {
    selected.value = null;
    diff.value = "";
    notice.value = null;
    void refresh();
  },
);
</script>

<template>
  <div class="git-panel">
    <div class="git-bar">
      <span class="git-title">{{ t("git.title") }}</span>
      <select
        v-if="status?.branch"
        v-model="branch"
        class="branch-select"
        :title="t('git.branchSwitch')"
        :disabled="busy || branches.length === 0"
        @change="checkoutBranch"
      >
        <option v-for="name in branches" :key="name" :value="name">{{ name }}</option>
        <option v-if="!branches.includes(status.branch)" :value="status.branch">
          {{ status.branch }}
        </option>
      </select>
      <span v-if="status?.head" class="git-head">· {{ status.head }}</span>
      <span class="spacer"></span>
      <button
        class="icon-button"
        type="button"
        :title="t('git.refresh')"
        :disabled="loading || busy"
        @click="refresh"
      >
        <Icon name="reload" :size="13" />
      </button>
      <button
        class="mini-text"
        type="button"
        :title="t('git.pullHint')"
        :disabled="busy || !status"
        @click="pull"
      >
        {{ t("git.pull") }}
      </button>
      <button
        class="mini-text"
        type="button"
        :disabled="busy || !status || status.entries.length === 0"
        @click="stageAll"
      >
        {{ t("git.stageAll") }}
      </button>
    </div>

    <p v-if="errorText" class="git-message error">{{ errorText }}</p>
    <template v-else-if="status">
      <div class="commit-box">
        <input
          v-model="message"
          class="commit-input"
          type="text"
          :placeholder="t('git.commitPlaceholder')"
          data-shortcut-passthrough
          :disabled="busy"
          @keydown.enter.prevent="commit"
        />
        <button
          class="commit-button"
          type="button"
          :disabled="busy || !message.trim() || !hasStaged"
          @click="commit"
        >
          {{ t("git.commit") }}
        </button>
      </div>
      <p class="git-hint">
        {{ hasStaged ? t("git.commitHint") : t("git.noStaged") }}
      </p>
      <p v-if="notice" class="git-message notice">{{ notice }}</p>

      <p v-if="status.truncated" class="git-empty warn">
        {{ t("git.truncated", { shown: status.entries.length, total: status.total }) }}
      </p>
      <p v-if="status.entries.length === 0" class="git-empty">{{ t("git.clean") }}</p>

      <div v-else class="git-body">
        <div class="git-files">
          <section v-if="staged.length" class="git-group">
            <h3>{{ t("git.staged") }} <span class="count">{{ groupTitle(staged) }}</span></h3>
            <div v-for="entry in staged" :key="`s:${entry.path}`" class="git-row">
              <button
                class="git-file"
                :class="{ active: selected?.path === entry.path && selected?.staged }"
                type="button"
                @click="loadDiff(entry)"
              >
                <span class="git-code" :class="{ conflict: entry.conflicted }">{{ entry.code }}</span>
                <span class="git-path">{{ entry.path }}</span>
              </button>
              <button
                class="git-action"
                type="button"
                :title="t('git.unstage')"
                :disabled="busy"
                @click="toggleStage(entry)"
              >
                −
              </button>
            </div>
          </section>

          <section v-if="unstaged.length" class="git-group">
            <h3>{{ t("git.unstaged") }} <span class="count">{{ groupTitle(unstaged) }}</span></h3>
            <div v-for="entry in unstaged" :key="`u:${entry.path}`" class="git-row">
              <button
                class="git-file"
                :class="{ active: selected?.path === entry.path && !selected?.staged }"
                type="button"
                @click="loadDiff(entry)"
              >
                <span class="git-code" :class="{ conflict: entry.conflicted }">{{ entry.code }}</span>
                <span class="git-path">{{ entry.path }}</span>
              </button>
              <button
                class="git-action"
                type="button"
                :title="t('git.stage')"
                :disabled="busy"
                @click="toggleStage(entry)"
              >
                +
              </button>
            </div>
          </section>

          <section v-if="untracked.length" class="git-group">
            <h3>{{ t("git.untracked") }} <span class="count">{{ groupTitle(untracked) }}</span></h3>
            <div v-for="entry in untracked" :key="`n:${entry.path}`" class="git-row">
              <button
                class="git-file"
                :class="{ active: selected?.path === entry.path && selected?.untracked }"
                type="button"
                @click="loadDiff(entry)"
              >
                <span class="git-code untracked">{{ entry.code }}</span>
                <span class="git-path">{{ entry.path }}</span>
              </button>
              <button
                class="git-action"
                type="button"
                :title="t('git.stage')"
                :disabled="busy"
                @click="toggleStage(entry)"
              >
                +
              </button>
            </div>
          </section>
        </div>

        <div class="git-diff">
          <div v-if="selected" class="diff-head">
            <span class="diff-path">{{ selected.path }}</span>
            <span class="diff-label">
              {{ selected.staged ? t("git.staged") : selected.untracked ? t("git.untracked") : t("git.unstaged") }}
            </span>
          </div>
          <p v-if="diffLoading" class="git-hint">{{ t("git.loadingDiff") }}</p>
          <div v-else-if="selected && diff" class="diff-text">
            <div
              v-for="(line, index) in diffLines"
              :key="index"
              class="diff-line"
              :class="line.kind"
            >{{ line.text || " " }}</div>
          </div>
          <p v-else class="git-hint">{{ t("git.noSelection") }}</p>
        </div>
      </div>
    </template>
    <p v-else class="git-hint">{{ t("git.loading") }}</p>
  </div>
</template>

<style scoped>
.git-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  padding: 8px;
  gap: 6px;
}

.git-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 26px;
  color: var(--text-muted);
  font-size: 11px;
}

.git-title {
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.git-branch {
  overflow: hidden;
  color: var(--text);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.branch-select {
  max-width: 140px;
  height: 24px;
  padding: 0 4px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text);
  font: inherit;
  font-size: 11px;
}

.branch-select:disabled {
  opacity: 0.6;
}

.git-head {
  color: var(--text-muted);
}

.spacer {
  flex: 1;
}

.icon-button {
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-muted);
}

.icon-button:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.commit-box {
  display: flex;
  gap: 6px;
}

.commit-input {
  flex: 1;
  min-width: 0;
  height: 30px;
  padding: 0 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text);
  font: inherit;
  font-size: 12px;
}

.commit-input:focus {
  border-color: var(--accent);
  outline: none;
}

.commit-button {
  height: 30px;
  padding: 0 10px;
  border: 1px solid var(--accent);
  border-radius: var(--radius-sm);
  background: var(--accent);
  color: var(--bg-app);
  font-size: 12px;
  font-weight: 600;
}

.commit-button:disabled {
  cursor: default;
  opacity: 0.45;
}

.git-hint {
  margin: 0;
  color: var(--text-muted);
  font-size: 11px;
}

.git-message {
  margin: 0;
  padding: 6px 8px;
  border-radius: var(--radius-sm);
  background: var(--bg-elevated);
  font-size: 11px;
  white-space: pre-wrap;
}

.git-message.error {
  color: var(--danger);
}

.git-message.notice {
  color: var(--accent);
}

.git-empty {
  margin: 12px 0;
  color: var(--text-muted);
  font-size: 12px;
}

.git-body {
  display: flex;
  flex-direction: column;
  min-height: 0;
  flex: 1;
  gap: 6px;
}

.git-files {
  max-height: 42%;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
}

.git-group h3 {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 0;
  padding: 5px 8px;
  border-bottom: 1px solid var(--border);
  color: var(--text-muted);
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.git-group + .git-group h3 {
  border-top: 1px solid var(--border);
}

.count {
  color: var(--text-muted);
  font-weight: 400;
}

.git-row {
  display: flex;
  align-items: stretch;
  border-bottom: 1px solid var(--border);
}

.git-row:last-child {
  border-bottom: none;
}

.git-file {
  display: flex;
  align-items: center;
  gap: 7px;
  min-width: 0;
  flex: 1;
  padding: 5px 8px;
  border: none;
  background: transparent;
  color: var(--text);
  font-size: 12px;
  text-align: left;
}

.git-file:hover,
.git-file.active {
  background: var(--bg-hover);
}

.git-path {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.git-code {
  flex: none;
  min-width: 18px;
  color: var(--accent);
  font-family: var(--mono);
  font-size: 11px;
  font-weight: 600;
}

.git-code.untracked {
  color: var(--text-muted);
}

.git-code.conflict {
  color: var(--danger);
}

.git-action {
  width: 28px;
  border: none;
  border-left: 1px solid var(--border);
  background: transparent;
  color: var(--text-muted);
  font-size: 15px;
}

.git-action:hover:not(:disabled) {
  background: var(--bg-hover);
  color: var(--text);
}

.git-action:disabled {
  opacity: 0.4;
}

.git-diff {
  display: flex;
  flex-direction: column;
  min-height: 0;
  flex: 1;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
}

.diff-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 8px;
  border-bottom: 1px solid var(--border);
  font-size: 11px;
}

.diff-path {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.diff-label {
  margin-left: auto;
  color: var(--text-muted);
}

.diff-text {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 4px 0;
  font-family: var(--mono);
  font-size: 11px;
  line-height: 1.45;
}

.diff-line {
  padding: 0 8px;
  white-space: pre;
}

.diff-line.add {
  background: rgba(78, 201, 176, 0.1);
  color: #4ec9b0;
}

.diff-line.del {
  background: rgba(244, 135, 113, 0.1);
  color: #f48771;
}

.diff-line.hunk {
  color: var(--accent);
}

.diff-line.meta {
  color: var(--text-muted);
}
</style>
