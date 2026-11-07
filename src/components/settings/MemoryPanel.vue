<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import Icon from "../Icon.vue";
import { t } from "../../i18n";

/**
 * Seeky's long-term memory, seen from the user's side.
 *
 * It lives in the settings rather than on the work area because it is not a tool: it is the
 * place where the user corrects what their partner believes about them. Nothing here deletes
 * a row — an entry the user takes out is retired and can be put back.
 *
 * The entries are cards with a heading and a short preview; the whole text opens in a modal
 * rather than pushing the rest of the list down. The search field filters the cards without
 * touching the database; the full text and the project path are part of the match.
 */
interface MemoryRow {
  id: number;
  kind: string;
  scope: string;
  title: string | null;
  text: string;
  importance: number;
  createdAt: number;
  origin: string;
  editedAt: number | null;
  retiredAt: number | null;
  workspace: string;
  /** `verified` when a tool result backed the note, `assumed` when it was reasoned. */
  basis: string;
  /** `work` or `private` — see the recall block. */
  class: string;
}

interface MemoryReport {
  total: number;
  duplicates: number[];
  stale: number[];
}

const rows = ref<MemoryRow[]>([]);
const retired = ref<MemoryRow[]>([]);
const query = ref("");
const report = ref<MemoryReport | null>(null);

/**
 * What the daily pass found last time. It runs in the background while the app is open, costs
 * nothing and changes nothing — this is only what it counted, so rot is noticed without a model
 * call and without touching a single note.
 */
interface MemoryCheck {
  day: string;
  total: number;
  duplicates: number;
  stale: number;
}
const lastCheck = ref<MemoryCheck | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);

/** The entry whose full text is open, and the edit drafts for it. */
const selectedId = ref<number | null>(null);
const selected = computed(() => rows.value.find((row) => row.id === selectedId.value) ?? null);
const editing = ref(false);
const draftTitle = ref("");
const draftText = ref("");
const retiredOpen = ref(false);

/** A memory the user writes is theirs from the start — no tool, no model in between. */
const newTitle = ref("");
const newMemory = ref("");
const adding = ref(false);

const TITLE_MAX = 120;

/** Old rows have no heading; their first line stands in until someone gives them one. */
function titleFor(row: MemoryRow): string {
  const stored = row.title?.trim();
  if (stored) return stored;
  const line = row.text
    .split("\n")
    .map((entry) => entry.trim())
    .find((entry) => entry.length > 0) ?? "";
  const flat = line.replace(/\s+/g, " ").trim();
  return flat.length > TITLE_MAX ? `${flat.slice(0, TITLE_MAX - 1)}…` : flat;
}

function openEntry(row: MemoryRow): void {
  selectedId.value = row.id;
  editing.value = false;
}

const needle = computed(() => query.value.trim().toLowerCase());

function matches(row: MemoryRow): boolean {
  if (!needle.value) return true;
  return [titleFor(row), row.text, row.kind, row.scope, row.workspace]
    .join("\n")
    .toLowerCase()
    .includes(needle.value);
}

const visibleRows = computed(() => rows.value.filter(matches));
const visibleRetired = computed(() => retired.value.filter(matches));

function closeEntry(): void {
  selectedId.value = null;
  editing.value = false;
}

async function addMemory() {
  const text = newMemory.value.trim();
  if (!text) return;
  adding.value = true;
  try {
    await invoke("memory_add", { text, title: newTitle.value.trim() });
    newMemory.value = "";
    newTitle.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    adding.value = false;
  }
  await load();
}

const scopeLabels = computed<Record<string, string>>(() => ({
  user: t("memory.scopeUser"),
  agent: t("memory.scopeAgent"),
  session: t("memory.scopeSession"),
}));

/** The sphere is what decides whether a note may ever leave the conversation. */
const classLabels = computed<Record<string, string>>(() => ({
  work: t("memory.classWork"),
  private: t("memory.classPrivate"),
}));

async function setClass(row: MemoryRow, value: string): Promise<void> {
  if (value === row.class) return;
  try {
    await invoke("memory_update", { id: row.id, class: value });
  } catch (cause) {
    error.value = String(cause);
  }
  await load();
}

async function load() {
  loading.value = true;
  error.value = null;
  try {
    const [active, aside] = await Promise.all([
      invoke<MemoryRow[]>("memories_list", { limit: 200 }),
      invoke<MemoryRow[]>("memories_retired", { limit: 200 }),
    ]);
    rows.value = active;
    retired.value = aside;
  } catch (cause) {
    error.value = String(cause);
    rows.value = [];
    retired.value = [];
  } finally {
    loading.value = false;
  }
}

function startEdit(row: MemoryRow) {
  selectedId.value = row.id;
  draftTitle.value = titleFor(row);
  draftText.value = row.text;
  editing.value = true;
}

function cancelEdit() {
  editing.value = false;
}

/** Saving a correction is the one thing that marks an entry as the user's own. */
async function saveEdit() {
  const row = selected.value;
  if (!row) return;
  const text = draftText.value.trim();
  if (!text) return;
  try {
    await invoke("memory_update", {
      id: row.id,
      title: draftTitle.value.trim(),
      text,
    });
    editing.value = false;
  } catch (cause) {
    error.value = String(cause);
  }
  await load();
}

async function retire(id: number) {
  try {
    await invoke("memory_retire", { id });
  } catch (cause) {
    error.value = String(cause);
  }
  if (selectedId.value === id) closeEntry();
  await load();
}

async function restore(id: number) {
  try {
    await invoke("memory_restore", { id });
  } catch (cause) {
    error.value = String(cause);
  }
  await load();
}

function formatDate(seconds: number): string {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "short",
    timeStyle: "short",
  }).format(new Date(seconds * 1000));
}

/** The "doctor" pass: mechanical findings only. Setting entries aside stays a human decision. */
async function check() {
  try {
    report.value = await invoke<MemoryReport>("memory_report");
  } catch {
    report.value = null;
  }
}

async function prune(ids: number[]) {
  if (ids.length === 0) return;
  await invoke("memory_prune", { ids });
  report.value = null;
  await load();
}

function onKey(event: KeyboardEvent): void {
  if (event.key === "Escape" && selectedId.value !== null) closeEntry();
}

async function loadLastCheck(): Promise<void> {
  try {
    lastCheck.value = await invoke<MemoryCheck | null>("memory_check_status");
  } catch {
    lastCheck.value = null;
  }
}

onMounted(() => {
  void load();
  void loadLastCheck();
  window.addEventListener("keydown", onKey);
});

onBeforeUnmount(() => window.removeEventListener("keydown", onKey));
</script>

<template>
  <div class="memory">
    <header class="head">
      <p class="hint">{{ t("memory.hint") }}</p>
      <div class="actions">
        <button class="mini" type="button" :title="t('memory.check')" @click="check">
          <Icon name="sparkle" :size="14" />
        </button>
        <button class="mini" type="button" :title="t('memory.refresh')" @click="load">
          <Icon name="reload" :size="14" />
        </button>
      </div>
    </header>

    <label class="search">
      <Icon name="search" :size="13" />
      <input
        v-model="query"
        type="search"
        :placeholder="t('memory.search')"
        :aria-label="t('memory.search')"
      />
    </label>

    <!-- The nightly pass found something: name it, and let the check button show the list. -->
    <p v-if="lastCheck && (lastCheck.duplicates || lastCheck.stale)" class="hint nightly">
      {{
        t("memory.nightlyFound", {
          day: lastCheck.day,
          duplicates: lastCheck.duplicates,
          stale: lastCheck.stale,
        })
      }}
    </p>

    <div v-if="report" class="report">
      <div v-if="report.duplicates.length || report.stale.length">
        <div v-if="report.duplicates.length" class="report-row">
          <span>{{ t("memory.duplicates", { count: report.duplicates.length }) }}</span>
          <button class="mini-text" type="button" @click="prune(report.duplicates)">
            {{ t("memory.retire") }}
          </button>
        </div>
        <div v-if="report.stale.length" class="report-row">
          <span>{{ t("memory.stale", { count: report.stale.length }) }}</span>
          <button class="mini-text" type="button" @click="prune(report.stale)">
            {{ t("memory.retire") }}
          </button>
        </div>
      </div>
      <p v-else class="report-clean">{{ t("memory.clean") }}</p>
    </div>

    <div class="add">
      <input        v-model="newTitle"
        class="title-input"
        type="text"
        :maxlength="TITLE_MAX"
        :placeholder="t('memory.titlePlaceholder')"
      />
      <textarea
        v-model="newMemory"
        class="editor"
        rows="2"
        :placeholder="t('memory.addPlaceholder')"
        @keydown.ctrl.enter.prevent="addMemory"
      ></textarea>
      <div class="editor-actions">
        <span class="counter">{{ newTitle.length }}/{{ TITLE_MAX }}</span>
        <button
          class="mini-text primary"
          type="button"
          :disabled="adding || !newMemory.trim()"
          @click="addMemory"
        >
          {{ t("memory.add") }}
        </button>
      </div>
    </div>

    <p v-if="loading" class="note">{{ t("viewFile.loading") }}</p>
    <p v-else-if="error" class="note error">{{ error }}</p>
    <p v-else-if="rows.length === 0" class="note">{{ t("memory.empty") }}</p>
    <p v-else-if="visibleRows.length === 0" class="note">{{ t("memory.noMatch") }}</p>

    <ul v-else class="grid">
      <li v-for="row in visibleRows" :key="row.id" class="memory-card">
        <button
          class="memory-card-main"
          type="button"
          :title="t('memory.open')"
          @click="openEntry(row)"
        >
          <span class="memory-title">{{ titleFor(row) }}</span>
          <span class="entry-chips">
            <span class="chip scope">{{ scopeLabels[row.scope] ?? row.scope }}</span>
            <span
              v-if="row.class !== 'work'"
              class="chip sphere"
              :class="row.class"
              :title="t('memory.classHint')"
            >
              {{ classLabels[row.class] ?? row.class }}
            </span>
            <span v-if="row.workspace" class="chip project" :title="row.workspace">
              {{ t("memory.project") }}
            </span>
            <span class="chip kind">{{ row.kind }}</span>
            <span v-if="row.origin === 'user'" class="chip mine">
              {{ row.editedAt ? t("memory.changedByUser") : t("memory.writtenByUser") }}
            </span>
            <!-- The same judgement the model sees: a note it reasoned up is not a fact. -->
            <span
              v-else-if="row.basis === 'assumed'"
              class="chip unverified"
              :title="t('memory.unverifiedHint')"
            >
              {{ t("memory.unverified") }}
            </span>
          </span>
          <span class="memory-preview">{{ row.text }}</span>
        </button>
        <div class="memory-foot">
          <span class="when">{{ formatDate(row.editedAt ?? row.createdAt) }}</span>
          <button class="mini" type="button" :title="t('memory.edit')" @click="startEdit(row)">
            <Icon name="pencil" :size="12" />
          </button>
          <button class="mini" type="button" :title="t('memory.retire')" @click="retire(row.id)">
            <Icon name="x" :size="12" />
          </button>
        </div>
      </li>
    </ul>

    <section v-if="visibleRetired.length" class="retired">
      <button class="retired-head" type="button" @click="retiredOpen = !retiredOpen">
        <Icon :name="retiredOpen ? 'chevron-down' : 'chevron-right'" :size="12" />
        <span>{{ t("memory.retired", { count: visibleRetired.length }) }}</span>
      </button>
      <p v-if="retiredOpen" class="note">{{ t("memory.retiredHint") }}</p>
      <ul v-if="retiredOpen" class="list">
        <li v-for="row in visibleRetired" :key="row.id" class="entry aside">
          <div class="entry-head">
            <span class="chip scope">{{ scopeLabels[row.scope] ?? row.scope }}</span>
            <span v-if="row.workspace" class="chip project" :title="row.workspace">
              {{ t("memory.project") }}
            </span>
            <span class="chip kind">{{ row.kind }}</span>
            <span class="when">{{ formatDate(row.retiredAt ?? row.createdAt) }}</span>
            <button class="mini-text" type="button" @click="restore(row.id)">
              {{ t("memory.restore") }}
            </button>
          </div>
          <p class="text">{{ row.text }}</p>
        </li>
      </ul>
    </section>

    <!-- The full text floats above the grid: opening an entry must not push the cards
         below it around, and the list can stay dense. -->
    <Teleport to="body">
      <div v-if="selected" class="modal-backdrop" @click.self="closeEntry">
        <div class="modal" role="dialog" aria-modal="true">
          <header class="modal-head">
            <div class="modal-heading">
              <h3 class="modal-title">{{ titleFor(selected) }}</h3>
              <div class="entry-chips">
                <span class="chip scope">{{ scopeLabels[selected.scope] ?? selected.scope }}</span>
                <span
                  v-if="selected.class !== 'work'"
                  class="chip sphere"
                  :class="selected.class"
                  :title="t('memory.classHint')"
                >
                  {{ classLabels[selected.class] ?? selected.class }}
                </span>
                <span v-if="selected.workspace" class="chip project" :title="selected.workspace">
                  {{ t("memory.project") }}
                </span>
                <span class="chip kind">{{ selected.kind }}</span>
                <span v-if="selected.origin === 'user'" class="chip mine">
                  {{
                    selected.editedAt
                      ? t("memory.changedByUser")
                      : t("memory.writtenByUser")
                  }}
                </span>
              </div>
            </div>
            <button class="mini" type="button" :title="t('common.close')" @click="closeEntry">
              <Icon name="x" :size="14" />
            </button>
          </header>

          <div v-if="editing" class="modal-body">
            <input
              v-model="draftTitle"
              class="title-input"
              type="text"
              :maxlength="TITLE_MAX"
              :placeholder="t('memory.titlePlaceholder')"
            />
            <textarea v-model="draftText" class="editor" rows="8"></textarea>
          </div>
          <div v-else class="modal-body full">{{ selected.text }}</div>

          <div class="sphere-row">
            <span class="label">{{ t("memory.classLabel") }}</span>
            <select
              class="sphere-select"
              :value="selected.class"
              :title="t('memory.classHint')"
              @change="setClass(selected, ($event.target as HTMLSelectElement).value)"
            >
              <option value="work">{{ t("memory.classWork") }}</option>
              <option value="private">{{ t("memory.classPrivate") }}</option>
            </select>
            <span class="sphere-hint">{{ t("memory.classHint") }}</span>
          </div>

          <footer class="modal-foot">
            <template v-if="editing">
              <button class="mini-text" type="button" @click="cancelEdit">
                {{ t("memory.cancel") }}
              </button>
              <button
                class="mini-text primary"
                type="button"
                :disabled="!draftText.trim()"
                @click="saveEdit"
              >
                {{ t("memory.save") }}
              </button>
            </template>
            <template v-else>
              <span class="modal-date">
                {{ formatDate(selected.editedAt ?? selected.createdAt) }}
              </span>
              <button class="mini-text" type="button" @click="startEdit(selected)">
                {{ t("memory.edit") }}
              </button>
              <button class="mini-text" type="button" @click="retire(selected.id)">
                {{ t("memory.retire") }}
              </button>
            </template>
          </footer>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.memory {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
}

.hint {
  margin: 0;
  color: var(--text-muted);
  font-size: 12px;
}

.search {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 30px;
  padding: 0 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text-muted);
}

.search input {
  flex: 1;
  min-width: 0;
  border: none;
  background: transparent;
  color: var(--text);
  font: inherit;
  font-size: 12px;
  outline: none;
}

.actions {
  display: flex;
  gap: 4px;
  flex: none;
}

.mini {
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  flex: none;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-muted);
}

.mini:hover {
  background: var(--bg-hover);
  color: var(--text-strong);
}

.note {
  margin: 0;
  color: var(--text-muted);
  font-size: 12px;
}

.note.error {
  color: var(--danger);
}

.report {
  padding: 10px 12px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  background: var(--bg-panel);
}

.report-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  color: var(--text);
  font-size: 12px;
}

.report-row + .report-row {
  margin-top: 8px;
}

.report-clean {
  margin: 0;
  color: var(--text-muted);
  font-size: 12px;
}

.mini-text {
  padding: 4px 10px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text);
  font-size: 11px;
}

.mini-text:hover {
  border-color: var(--accent);
  color: var(--text-strong);
}

.mini-text.primary {
  border-color: var(--accent-strong);
  background: var(--accent-strong);
  color: var(--accent-contrast);
}

.add {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 10px 12px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  background: var(--bg-panel);
}

.mini-text:disabled {
  opacity: 0.5;
}

.list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.entry {
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-panel);
}

.entry.aside {
  opacity: 0.7;
}

.entry-head {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 6px;
}

.chip {
  padding: 1px 7px;
  border-radius: 999px;
  font-size: 11px;
}

.chip.sphere.private {
  border-color: var(--accent-strong);
  color: var(--accent);
}

.sphere-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px 10px;
  padding: 10px 18px;
  border-top: 1px solid var(--border);
}

.sphere-row .label {
  color: var(--text-muted);
  font-size: 12px;
}

.sphere-select {
  height: 28px;
  padding: 0 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text);
  font-size: 12px;
}

.sphere-hint {
  flex: 1;
  min-width: 180px;
  color: var(--text-muted);
  font-size: 11px;
  line-height: 1.4;
}

.chip.scope {
  background: var(--accent-strong);
  color: var(--accent-contrast);
}

.chip.kind {
  border: 1px solid var(--border-strong);
  color: var(--text-muted);
}

.chip.project {
  border: 1px solid var(--accent);
  color: var(--accent);
}

.nightly {
  margin: 0 0 10px;
}

.chip.unverified {
  border-color: var(--border-strong);
  color: var(--text-muted);
  border-style: dashed;
}

.chip.mine {
  background: var(--bg-elevated);
  color: var(--text-strong);
}

.when {
  margin-left: auto;
  color: var(--text-muted);
  font-size: 11px;
}

.text {
  margin: 0;
  color: var(--text);
  font-size: 12px;
  line-height: 1.5;
}

.editor {
  width: 100%;
  padding: 6px 8px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  background: var(--bg);
  color: var(--text);
  font-family: inherit;
  font-size: 12px;
  line-height: 1.5;
  resize: vertical;
}

.editor-actions {
  display: flex;
  justify-content: flex-end;
  gap: 6px;
  margin-top: 6px;
}

.retired {
  margin-top: 6px;
}

.retired-head {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 0;
  border: none;
  background: transparent;
  color: var(--text-muted);
  font-size: 12px;
}

.retired-head:hover {
  color: var(--text-strong);
}

.retired .note {
  margin-bottom: 8px;
}

.title-input {
  width: 100%;
  height: 30px;
  padding: 0 8px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  background: var(--bg);
  color: var(--text);
  font-family: inherit;
  font-size: 12px;
}

.title-input:focus,
.editor:focus {
  outline: none;
  border-color: var(--accent);
}

.counter {
  margin-right: auto;
  color: var(--text-muted);
  font-size: 11px;
}

/* The list is a grid so the width of the settings page is used instead of one tall
   column. Each card carries a heading, a short preview and the actions. */
.grid {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 10px;
  align-items: start;
}

.memory-card {
  display: flex;
  flex-direction: column;
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-panel);
  overflow: hidden;
}

.memory-card-main {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: 7px;
  flex: 1;
  min-width: 0;
  padding: 11px 12px;
  border: none;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.memory-card-main:hover {
  background: var(--bg-hover);
}

.memory-title {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  overflow: hidden;
  color: var(--text-strong);
  font-size: 13px;
  font-weight: 600;
  line-height: 1.35;
  word-break: break-word;
}

.entry-chips {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}

.memory-preview {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 3;
  overflow: hidden;
  color: var(--text-muted);
  font-size: 12px;
  line-height: 1.45;
  word-break: break-word;
}

.memory-foot {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 5px 8px 5px 12px;
  border-top: 1px solid var(--border);
}

/* The whole text floats above the grid. */
.modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 60;
  display: grid;
  place-items: center;
  padding: 32px;
  background: rgb(0 0 0 / 55%);
}

.modal {
  display: flex;
  flex-direction: column;
  width: min(760px, 100%);
  max-height: min(82vh, 760px);
  border: 1px solid var(--border-strong);
  border-radius: 12px;
  background: var(--bg-panel);
  box-shadow: 0 24px 70px rgb(0 0 0 / 55%);
  overflow: hidden;
}

.modal-head {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 16px 18px;
  border-bottom: 1px solid var(--border);
}

.modal-heading {
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex: 1;
  min-width: 0;
}

.modal-title {
  margin: 0;
  color: var(--text-strong);
  font-size: 16px;
  line-height: 1.35;
  word-break: break-word;
}

.modal-body {
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex: 1;
  min-height: 0;
  padding: 16px 18px;
  overflow-y: auto;
}

.modal-body.full {
  color: var(--text);
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
}

.modal-body .editor {
  flex: 1;
  min-height: 180px;
}

.modal-foot {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 18px;
  border-top: 1px solid var(--border);
}

.modal-date {
  margin-right: auto;
  color: var(--text-muted);
  font-size: 11px;
}

@media (max-width: 700px) {
  .modal-backdrop {
    padding: 12px;
  }
}
</style>
