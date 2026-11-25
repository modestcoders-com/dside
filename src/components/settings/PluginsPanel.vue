<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import Icon from "../Icon.vue";
import { t } from "../../i18n";
import { settings } from "../../stores/settings";

/**
 * Plugins: folders the user adds, each one a manifest, optional notes and an optional program.
 *
 * The panel says exactly what a click will do. A plugin is enabled *for this project* — the same
 * plugin may be on in one folder and off in another — and turning it off never deletes anything.
 * The hash is shown because a replaced binary is otherwise indistinguishable from a trusted one.
 */
interface PluginInfo {
  id: string;
  name: string;
  version: string;
  author: string;
  description: string;
  path: string;
  entry: string | null;
  tools: string[];
  toolNames: string[];
  hasNotes: boolean;
  hash: string | null;
  error: string | null;
}

interface ProbeReport {
  offered: string[];
  allowed: string[];
  refused: string[];
  missing: string[];
}

const plugins = ref<PluginInfo[]>([]);
const loading = ref(false);
const loadError = ref<string | null>(null);
const rootPath = ref("");
const probing = ref<string | null>(null);
const reports = ref<Record<string, ProbeReport | string>>({});

/** The same normalisation the engine applies: trailing separators do not make a second project. */
const projectKey = computed(() => settings.workspace.trim().replace(/[\\/]+$/, ""));
const hasProject = computed(() => projectKey.value.length > 0);

function enabledIds(): string[] {
  return settings.enabledPlugins[projectKey.value] ?? [];
}

function isEnabled(id: string): boolean {
  return enabledIds().includes(id);
}

function toggle(id: string, on: boolean): void {
  if (!hasProject.value) return;
  const ids = new Set(enabledIds());
  if (on) ids.add(id);
  else ids.delete(id);
  settings.enabledPlugins = { ...settings.enabledPlugins, [projectKey.value]: [...ids] };
}

async function refresh(): Promise<void> {
  loading.value = true;
  loadError.value = null;
  try {
    plugins.value = await invoke<PluginInfo[]>("plugins_list", {
      root: settings.pluginRoot || null,
    });
    rootPath.value = await invoke<string>("plugins_root");
  } catch (error) {
    loadError.value = String(error);
    plugins.value = [];
  } finally {
    loading.value = false;
  }
}

async function openRoot(): Promise<void> {
  try {
    await invoke("plugins_open_root");
  } catch (error) {
    loadError.value = String(error);
  }
}

/** Starts the server once and reports what it offers, so "does it work" is not a guess. */
async function probe(id: string): Promise<void> {
  probing.value = id;
  try {
    reports.value[id] = await invoke<ProbeReport>("plugins_probe", {
      root: settings.pluginRoot || null,
      id,
    });
  } catch (error) {
    reports.value[id] = String(error);
  } finally {
    probing.value = null;
  }
}

function report(id: string): ProbeReport | string | null {
  return reports.value[id] ?? null;
}

function isReport(value: ProbeReport | string): value is ProbeReport {
  return typeof value === "object";
}

onMounted(refresh);
</script>

<template>
  <div class="plugins">
    <p class="hint">{{ t("plugins.hint") }}</p>

    <div class="card">
      <div class="row stacked">
        <div class="text">
          <div class="label">{{ t("plugins.root") }}</div>
          <div class="desc">{{ t("plugins.rootDesc") }}</div>
        </div>
        <div class="root-line">
          <code class="path" :title="rootPath">{{ rootPath || "…" }}</code>
          <button class="mini" type="button" :disabled="loading" @click="refresh">
            <Icon name="reload" :size="12" />
            <span>{{ t("plugins.reload") }}</span>
          </button>
          <button class="mini" type="button" @click="openRoot">
            <Icon name="folder" :size="12" />
            <span>{{ t("plugins.openFolder") }}</span>
          </button>
        </div>
      </div>

      <!-- A second folder for a shared or version-controlled collection. One root per
           installation: a project chooses which plugins are on, not where they come from. -->
      <div class="row stacked">
        <div class="text">
          <div class="label">{{ t("plugins.extraRoot") }}</div>
          <div class="desc">{{ t("plugins.extraRootDesc") }}</div>
        </div>
        <input
          v-model.trim="settings.pluginRoot"
          class="input"
          spellcheck="false"
          :placeholder="t('plugins.extraRootPlaceholder')"
          @change="refresh"
        />
      </div>
    </div>

    <p v-if="loadError" class="error">{{ loadError }}</p>
    <p v-if="!hasProject" class="note">{{ t("plugins.noProject") }}</p>
    <p v-else-if="plugins.length === 0 && !loading" class="note">
      {{ t("plugins.empty") }}
    </p>

    <div v-for="plugin in plugins" :key="plugin.id" class="card plugin">
      <div class="plugin-head">
        <div class="titles">
          <div class="title">
            {{ plugin.name || plugin.id }}
            <span v-if="plugin.version" class="version">{{ plugin.version }}</span>
          </div>
          <div class="sub">
            <code>{{ plugin.id }}</code>
            <span v-if="plugin.author"> · {{ plugin.author }}</span>
            <span v-if="plugin.hasNotes"> · {{ t("plugins.notes") }}</span>
            <span v-if="plugin.entry"> · {{ plugin.entry }}</span>
          </div>
        </div>
        <button
          class="switch"
          :class="{ on: isEnabled(plugin.id) }"
          type="button"
          role="switch"
          :aria-checked="isEnabled(plugin.id)"
          :aria-label="isEnabled(plugin.id) ? t('plugins.on') : t('plugins.off')"
          :title="isEnabled(plugin.id) ? t('plugins.on') : t('plugins.off')"
          :disabled="!hasProject || !!plugin.error"
          @click="toggle(plugin.id, !isEnabled(plugin.id))"
        >
          <span class="knob"></span>
        </button>
      </div>

      <p v-if="plugin.description" class="desc body">{{ plugin.description }}</p>

      <p v-if="plugin.error" class="error">{{ plugin.error }}</p>

      <div class="tools">
        <span class="tools-label">{{ t("plugins.tools") }}</span>
        <template v-if="plugin.toolNames.length > 0">
          <code v-for="name in plugin.toolNames" :key="name" class="tool">{{ name }}</code>
        </template>
        <span v-else class="muted">{{ t("plugins.noTools") }}</span>
      </div>

      <div class="foot">
        <code class="path" :title="plugin.path">{{ plugin.path }}</code>
        <code v-if="plugin.hash" class="hash" :title="t('plugins.hashHint')">{{ plugin.hash }}</code>
        <button
          v-if="plugin.entry"
          class="mini"
          type="button"
          :disabled="probing === plugin.id"
          @click="probe(plugin.id)"
        >
          <Icon name="activity" :size="12" />
          <span>{{ probing === plugin.id ? t("plugins.testing") : t("plugins.test") }}</span>
        </button>
      </div>

      <div v-if="report(plugin.id)" class="report">
        <template v-if="isReport(report(plugin.id)!)">
          <div class="report-line">
            <span class="report-label ok">{{ t("plugins.allowed") }}</span>
            <code v-for="name in (report(plugin.id) as ProbeReport).allowed" :key="name" class="tool">
              {{ name }}
            </code>
            <span
              v-if="(report(plugin.id) as ProbeReport).allowed.length === 0"
              class="muted"
            >—</span>
          </div>
          <div class="report-line">
            <span class="report-label">{{ t("plugins.refused") }}</span>
            <code v-for="name in (report(plugin.id) as ProbeReport).refused" :key="name" class="tool">
              {{ name }}
            </code>
            <span
              v-if="(report(plugin.id) as ProbeReport).refused.length === 0"
              class="muted"
            >—</span>
          </div>
          <div class="report-line">
            <span class="report-label">{{ t("plugins.missing") }}</span>
            <code v-for="name in (report(plugin.id) as ProbeReport).missing" :key="name" class="tool">
              {{ name }}
            </code>
            <span
              v-if="(report(plugin.id) as ProbeReport).missing.length === 0"
              class="muted"
            >—</span>
          </div>
        </template>
        <p v-else class="error">{{ report(plugin.id) }}</p>
      </div>
    </div>

    <p class="note">{{ t("plugins.note") }}</p>
  </div>
</template>

<style scoped>
.plugins {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.hint,
.note {
  margin: 0;
  color: var(--text-muted);
  font-size: 12px;
  line-height: 1.5;
}

.note {
  padding: 9px 11px;
  border-left: 2px solid var(--accent);
  background: var(--bg-panel);
}

.card {
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-panel);
}

.row {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 14px 16px;
}

.row + .row {
  border-top: 1px solid var(--border);
}

.row.stacked {
  flex-direction: column;
  align-items: stretch;
  gap: 8px;
}

.text {
  flex: 1;
  min-width: 0;
}

.label {
  color: var(--text-strong);
  font-size: 13px;
}

.desc {
  margin-top: 3px;
  color: var(--text-muted);
  font-size: 12px;
  line-height: 1.45;
}

.body {
  margin: 0;
  padding: 0 16px 12px;
}

.root-line {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.path {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-muted);
  font-family: var(--mono);
  font-size: 11px;
}

.input {
  height: 30px;
  padding: 0 9px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text);
  font-family: var(--mono);
  font-size: 11.5px;
}

.input:focus {
  border-color: var(--accent);
  outline: none;
}

.mini {
  display: inline-flex;
  flex: none;
  align-items: center;
  gap: 5px;
  height: 26px;
  padding: 0 9px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-muted);
  font-size: 11.5px;
}

.mini:hover:not(:disabled) {
  border-color: var(--accent);
  color: var(--accent);
}

.mini:disabled {
  cursor: default;
  opacity: 0.5;
}

.plugin-head {
  display: flex;
  align-items: flex-start;
  gap: 16px;
  padding: 14px 16px 10px;
}

.titles {
  flex: 1;
  min-width: 0;
}

.title {
  color: var(--text-strong);
  font-size: 13px;
  font-weight: 500;
}

.version {
  margin-left: 6px;
  color: var(--text-muted);
  font-family: var(--mono);
  font-size: 11px;
  font-weight: 400;
}

.sub {
  margin-top: 3px;
  color: var(--text-muted);
  font-family: var(--mono);
  font-size: 11px;
}

.toggle {
  flex: none;
  max-width: 190px;
}

.switch {
  position: relative;
  flex: none;
  width: 40px;
  height: 22px;
  margin-top: 2px;
  border: none;
  border-radius: 11px;
  background: var(--bg-active);
  transition: background 0.15s ease;
}

.switch.on {
  background: var(--accent-strong);
}

.switch:disabled {
  cursor: default;
  opacity: 0.45;
}

.knob {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--text-strong);
  transition: transform 0.15s ease;
}

.switch.on .knob {
  transform: translateX(18px);
  background: var(--accent-contrast);
}

.tools {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
  padding: 0 16px 12px;
}

.tools-label {
  color: var(--text-muted);
  font-size: 11.5px;
}

.tool {
  padding: 2px 6px;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: var(--bg-input);
  color: var(--text);
  font-family: var(--mono);
  font-size: 10.5px;
}

.muted {
  color: var(--text-muted);
  font-size: 11.5px;
}

.foot {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 16px;
  border-top: 1px solid var(--border);
}

.hash {
  flex: none;
  color: var(--text-muted);
  font-family: var(--mono);
  font-size: 10.5px;
}

.report {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 10px 16px 14px;
  border-top: 1px solid var(--border);
  background: var(--bg-input);
}

.report-line {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
}

.report-label {
  min-width: 74px;
  color: var(--text-muted);
  font-size: 11px;
}

.report-label.ok {
  color: var(--accent);
}

.error {
  margin: 0;
  padding: 0 16px 12px;
  color: var(--danger);
  font-size: 11.5px;
  line-height: 1.45;
}
</style>
