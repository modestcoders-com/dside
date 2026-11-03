<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import Icon from "../Icon.vue";
import ToggleRow from "./ToggleRow.vue";
import MemoryPanel from "./MemoryPanel.vue";
import PartnerPanel from "./PartnerPanel.vue";
import AboutPanel from "./AboutPanel.vue";
import DiagnosticsPanel from "./DiagnosticsPanel.vue";
import KeyboardPanel from "./KeyboardPanel.vue";
import RudderDialog from "../RudderDialog.vue";
import SandboxPanel from "./SandboxPanel.vue";
import WorkspacePicker from "../WorkspacePicker.vue";
import ConfirmDialog from "../ConfirmDialog.vue";
import { locale, localeLabel, locales, setLocale, t } from "../../i18n";
import type { Locale, MessageKey } from "../../i18n";
import type { PermissionMode } from "../../types";
import { links } from "../../engine/links";
import { settings } from "../../stores/settings";
import { activeProject, bindActiveThread } from "../../composables/useProjects";
import {
  balance,
  balanceError,
  loadBalance,
  effortsFor,
  loadModels,
  modelChoices,
  modelsError,
} from "../../stores/models";
import { settingsItem } from "../../stores/ui";
import { partnerDisplayName } from "../../stores/partner";
import {
  clearApiKey,
  engineReady,
  loadSessions,
  refreshSecret,
  saveApiKey,
  secretBackend,
} from "../../composables/useEngine";

const emit = defineEmits<{ close: [] }>();

const query = ref("");
const activeItem = ref<string>(settingsItem.value ?? "general");
const rudderOpen = ref(false);
const workspacePickerOpen = ref(false);
const projectsRootPickerOpen = ref(false);

/**
 * The folder is not a free-floating setting: the open thread moves with it, otherwise the
 * conversation and the tools would point at different projects until a restart fixed one.
 */
function chooseWorkspace(path: string): void {
  workspacePickerOpen.value = false;
  void bindActiveThread(path);
}

/** The base is a list source, not a working folder: nothing is reloaded except the rail. */
function chooseProjectsRoot(path: string): void {
  settings.projectsRoot = path;
  projectsRootPickerOpen.value = false;
}

const permissionModes: { value: PermissionMode; label: MessageKey; hint: MessageKey; icon: string }[] = [
  { value: "read", label: "permission.read", hint: "permission.readHint", icon: "eye" },
  { value: "ask", label: "permission.ask", hint: "permission.askHint", icon: "sparkle" },
  { value: "full", label: "permission.fullSetting", hint: "permission.fullHint", icon: "rudder" },
];

/** Choosing full in Settings goes through the same consent dialog as the composer. */
function choosePermission(mode: PermissionMode): void {
  if (mode === "full") {
    rudderOpen.value = true;
    return;
  }
  settings.permission = mode;
}

function confirmRudder(): void {
  settings.permission = "full";
  rudderOpen.value = false;
}

function cancelRudder(): void {
  rudderOpen.value = false;
}

interface NavItem {
  id: string;
  label: MessageKey;
  icon: string;
}

interface NavGroup {
  key: MessageKey;
  title?: string;
  items: NavItem[];
}

/**
 * The partner gets its own group instead of living under "Personalisation": that word means
 * look and feel to most people, while this is the page where the user changes what their
 * partner knows and is called.
 */
const groups = computed<NavGroup[]>(() => [
  {
    key: "settings.groups.preferences",
    items: [
      { id: "general", label: "settings.items.general", icon: "gear" },
      { id: "keyboard", label: "settings.items.keyboardShortcuts", icon: "grid" },
    ],
  },
  {
    key: "settings.groups.partner",
    title: partnerDisplayName.value.toUpperCase(),
    items: [
      { id: "memories", label: "settings.items.memories", icon: "sparkle" },
      { id: "partner", label: "settings.items.partner", icon: "user" },
    ],
  },
  {
    key: "settings.groups.management",
    items: [
      { id: "account", label: "settings.items.account", icon: "user" },
      { id: "sandbox", label: "settings.items.sandbox", icon: "shield" },
      { id: "diagnostics", label: "settings.items.diagnostics", icon: "monitor" },
      { id: "about", label: "settings.items.about", icon: "doc" },
    ],
  },
  {
    key: "settings.groups.archived",
    items: [{ id: "archived", label: "settings.items.archivedTasks", icon: "clock" }],
  },
]);

const filteredGroups = computed(() => {
  const needle = query.value.trim().toLowerCase();
  return groups.value
    .map((group) => ({
      key: group.key,
      title: group.title,
      items: group.items.filter((item) => t(item.label).toLowerCase().includes(needle)),
    }))
    .filter((group) => group.items.length > 0);
});

const modeCards = [
  {
    value: "coding",
    title: "settings.general.modeCodingTitle",
    description: "settings.general.modeCodingDesc",
    icon: "terminal",
  },
  {
    value: "everyday",
    title: "settings.general.modeEverydayTitle",
    description: "settings.general.modeEverydayDesc",
    icon: "user",
  },
] as const;

const appearanceOptions = [
  { value: "light", label: "settings.general.appearanceLight", icon: "sun" },
  { value: "dark", label: "settings.general.appearanceDark", icon: "moon" },
  { value: "system", label: "settings.general.appearanceSystem", icon: "monitor" },
] as const;

function changeLocale(event: Event) {
  const value = (event.target as HTMLSelectElement).value;
  setLocale(value as Locale);
}

const apiKeyDraft = ref("");

const secretHint = computed(() => {
  if (!engineReady.value) return t("model.notSet");
  return secretBackend.value === "keyring" ? t("model.storedKeyring") : t("model.storedFile");
});

async function saveKey() {
  if (!apiKeyDraft.value.trim()) return;
  await saveApiKey(apiKeyDraft.value);
  apiKeyDraft.value = "";
  // A fresh key usually means a fresh account: pull models and credits right away.
  // Serially on purpose: the provider's edge resets one of two simultaneous connections.
  await loadModels();
  await loadBalance();
}

async function removeKey() {
  await clearApiKey();
}

const systemName = ref("");
/** Tools the user allowed permanently; shown with a way back. */
const trustedTools = ref<string[]>([]);

async function loadTrustedTools(): Promise<void> {
  try {
    trustedTools.value = await invoke<string[]>("engine_trusted_tools");
  } catch {
    trustedTools.value = [];
  }
}

async function clearToolGrants(): Promise<void> {
  try {
    await invoke("engine_clear_tool_grants");
  } finally {
    trustedTools.value = [];
  }
}

/** True while the stored id is one the provider reports — then a picker makes sense. */
const modelIsKnown = computed(() => modelChoices().some((entry) => entry.id === settings.model));

async function openLink(url: string) {
  try {
    if (/^https?:/i.test(url)) await invoke("open_browser_window", { url });
    else await openUrl(url);
  } catch {
    // nothing sensible to do if even that fails
  }
}

onMounted(async () => {
  settingsItem.value = null;
  await refreshSecret();
  await loadTrustedTools();
  try {
    systemName.value = await invoke<string>("default_display_name");
  } catch {
    systemName.value = "";
  }
  if (engineReady.value) {
    await loadModels();
    await loadBalance();
  }
});

interface ArchivedSession {
  id: string;
  title: string;
  workspace: string;
  updatedAt: number;
}

const archived = ref<ArchivedSession[]>([]);
const archivedQuery = ref("");
const archivedProject = ref("");
/** Which session the confirmation is about; `"all"` means the whole archive. */
const archiveDeleteId = ref<string | null>(null);
const archivedBusy = ref(false);

/** The folders the archive actually contains, by their last path segment. */
const archivedProjects = computed(() => {
  const seen = new Map<string, string>();
  for (const session of archived.value) {
    if (!session.workspace || seen.has(session.workspace)) continue;
    seen.set(session.workspace, projectLabel(session.workspace));
  }
  return [...seen.entries()].map(([value, label]) => ({ value, label }));
});

function projectLabel(workspace: string): string {
  return workspace.replace(/[\\/]+$/, "").split(/[\\/]/).pop() || workspace;
}

const filteredArchived = computed(() => {
  const needle = archivedQuery.value.trim().toLowerCase();
  return archived.value.filter((session) => {
    if (archivedProject.value && session.workspace !== archivedProject.value) return false;
    if (!needle) return true;
    return (
      session.title.toLowerCase().includes(needle) ||
      session.workspace.toLowerCase().includes(needle)
    );
  });
});

function archivedDate(session: ArchivedSession): string {
  return new Date(session.updatedAt * 1000).toLocaleDateString();
}

async function loadArchived() {
  try {
    archived.value = await invoke<ArchivedSession[]>("sessions_archived");
  } catch {
    archived.value = [];
  }
}

async function restore(id: string) {
  await invoke("session_archive", { id, archived: false });
  // The rail keeps its own copy of the thread list. Refreshing only the archive left the
  // restored task out of the sidebar until the app was restarted.
  await Promise.all([loadArchived(), loadSessions()]);
}

/**
 * Deleting a conversation is not reversible, so it asks first — with the app's own dialog, the
 * same one the file history uses, not a browser confirm.
 */
async function deleteArchived(): Promise<void> {
  const target = archiveDeleteId.value;
  if (!target) return;
  archivedBusy.value = true;
  try {
    if (target === "all") {
      await invoke("session_delete_many", { ids: archived.value.map((session) => session.id) });
    } else {
      await invoke("session_delete", { id: target });
    }
  } finally {
    archivedBusy.value = false;
    archiveDeleteId.value = null;
    await loadArchived();
  }
}

const archiveDeleteTitle = computed(() =>
  archiveDeleteId.value === "all" ? t("archived.deleteAllTitle") : t("archived.deleteTitle"),
);

const archiveDeleteMessage = computed(() =>
  archiveDeleteId.value === "all"
    ? t("archived.confirmDeleteAll", { count: archived.value.length })
    : t("archived.confirmDelete"),
);

watch([archivedQuery, archivedProject], () => {
  archiveDeleteId.value = null;
});

// `immediate`: the page can open *on* this section (`openSettings("archived")`), and then
// there is no change to react to — the list stayed empty until the app was restarted.
watch(
  activeItem,
  (value) => {
    if (value === "archived") void loadArchived();
  },
  { immediate: true },
);
</script>

<template>
  <div class="settings">
    <aside class="nav card">
      <button class="back" type="button" @click="emit('close')">
        <Icon name="arrow-left" :size="14" />
        <span>{{ t("settings.backToApp") }}</span>
      </button>

      <div class="search">
        <Icon name="search" :size="14" />
        <input v-model="query" :placeholder="t('settings.search')" />
      </div>

      <div class="nav-scroll">
        <section v-for="group in filteredGroups" :key="group.key" class="nav-group">
          <div class="nav-group-title">{{ group.title ?? t(group.key) }}</div>
          <button
            v-for="item in group.items"
            :key="item.id"
            class="nav-item"
            :class="{ active: activeItem === item.id }"
            type="button"
            @click="activeItem = item.id"
          >
            <Icon :name="item.icon" :size="14" />
            <span>{{ t(item.label) }}</span>
          </button>
        </section>
      </div>
    </aside>

    <main class="content card">
      <template v-if="activeItem === 'general'">
        <h1 class="title">{{ t("settings.items.general") }}</h1>

        <h2 class="section">{{ t("settings.general.mode") }}</h2>
        <div class="mode-grid">
          <button
            v-for="card in modeCards"
            :key="card.value"
            class="mode-card"
            :class="{ active: settings.mode === card.value }"
            type="button"
            @click="settings.mode = card.value"
          >
            <Icon :name="card.icon" :size="18" />
            <div class="mode-text">
              <div class="mode-title">{{ t(card.title) }}</div>
              <div class="mode-desc">{{ t(card.description) }}</div>
            </div>
            <span class="radio" :class="{ on: settings.mode === card.value }"></span>
          </button>
        </div>

        <h2 class="section">{{ t("settings.general.application") }}</h2>
        <div class="card">
          <div class="row">
            <div class="text">
              <div class="label">{{ t("settings.general.appearance") }}</div>
              <div class="desc">{{ t("settings.general.appearanceDesc") }}</div>
            </div>
            <div class="segmented">
              <button
                v-for="option in appearanceOptions"
                :key="option.value"
                class="segment"
                :class="{ active: settings.appearance === option.value }"
                type="button"
                @click="settings.appearance = option.value"
              >
                <Icon :name="option.icon" :size="16" />
                <span>{{ t(option.label) }}</span>
              </button>
            </div>
          </div>

          <div class="row">
            <div class="text">
              <div class="label">{{ t("settings.general.readingSize") }}</div>
              <div class="desc">{{ t("settings.general.readingSizeDesc") }}</div>
            </div>
            <div class="sizer">
              <input v-model.number="settings.fontSize" type="range" min="11" max="20" step="1" />
              <span class="value">{{ settings.fontSize }} px</span>
            </div>
          </div>

          <div class="row">
            <div class="text">
              <div class="label">{{ t("settings.general.language") }}</div>
              <div class="desc">{{ t("settings.general.languageDesc") }}</div>
            </div>
            <select class="select" :value="locale" @change="changeLocale">
              <option v-for="code in locales" :key="code" :value="code">
                {{ localeLabel(code) }}
              </option>
            </select>
          </div>

          <div class="row">
            <div class="text">
              <div class="label">{{ t("settings.general.workspace") }}</div>
              <div class="desc">{{ t("settings.general.workspaceDesc") }}</div>
            </div>
            <div class="workspace">
              <input v-model="settings.workspace" class="input" />
              <button
                class="mini"
                type="button"
                :title="t('workspacePicker.chooseShort')"
                @click="workspacePickerOpen = true"
              >
                <Icon name="folder" :size="14" />
              </button>
              <button
                class="mini"
                type="button"
                :title="t('settings.general.activeProject')"
                @click="bindActiveThread(settings.workspace)"
              >
                <Icon name="reload" :size="14" />
              </button>
            </div>
          </div>

          <!-- The base is only listed, never written to: the active project above stays the
               one folder the tools may touch. -->
          <div class="row">
            <div class="text">
              <div class="label">{{ t("settings.general.projectsRoot") }}</div>
              <div class="desc">{{ t("settings.general.projectsRootDesc") }}</div>
            </div>
            <div class="workspace">
              <input v-model="settings.projectsRoot" class="input" />
              <button
                class="mini"
                type="button"
                :title="t('workspacePicker.chooseShort')"
                @click="projectsRootPickerOpen = true"
              >
                <Icon name="folder" :size="14" />
              </button>
            </div>
          </div>
          <p v-if="activeProject" class="hint">
            {{ t("settings.general.activeProject") }}: {{ activeProject }}
          </p>

          <ToggleRow
            v-model="settings.showInMenuBar"
            :label="t('settings.general.showInMenuBar')"
            :description="t('settings.general.showInMenuBarDesc')"
          />
          <ToggleRow
            v-model="settings.launchOnStartup"
            :label="t('settings.general.launchOnStartup')"
            :description="t('settings.general.launchOnStartupDesc')"
          />
          <ToggleRow
            v-model="settings.desktopNotifications"
            :label="t('settings.general.desktopNotifications')"
            :description="t('settings.general.desktopNotificationsDesc')"
          />
          <ToggleRow
            v-model="settings.followCompanion"
            :label="t('settings.general.followCompanion')"
            :description="t('settings.general.followCompanionDesc')"
          />
          <ToggleRow
            v-model="settings.browserControl"
            :label="t('settings.general.browserControl')"
            :description="t('settings.general.browserControlDesc')"
          />
          <ToggleRow
            v-model="settings.keepInBackground"
            :label="t('settings.general.keepInBackground')"
            :description="t('settings.general.keepInBackgroundDesc')"
          />
          <ToggleRow
            v-model="settings.joinEarlyAccess"
            :label="t('settings.general.joinEarlyAccess')"
            :description="t('settings.general.joinEarlyAccessDesc')"
          />
        </div>
      </template>

      <template v-else-if="activeItem === 'keyboard'">
        <h1 class="title">{{ t("settings.items.keyboardShortcuts") }}</h1>
        <KeyboardPanel />
      </template>

      <template v-else-if="activeItem === 'account'">
        <h1 class="title">{{ t("settings.items.account") }}</h1>

        <div class="card">
          <div class="row">
            <div class="text">
              <div class="label">{{ t("settings.general.displayName") }}</div>
              <div class="desc">{{ t("settings.general.displayNameDesc") }}</div>
            </div>
            <input v-model="settings.displayName" class="input" :placeholder="systemName" />
          </div>

          <div class="row">
            <div class="text">
              <div class="label">{{ t("account.provider") }}</div>
              <div class="desc">{{ t("model.balanceTotal") }}</div>
            </div>
            <div class="model-field">
              <span
                v-if="balance"
                class="value"
                :title="`${balance.toppedUp} + ${balance.granted} ${balance.currency}`"
              >
                {{ balance.total }} {{ balance.currency }}
              </span>
              <button
                class="mini"
                type="button"
                :title="t('account.usage')"
                @click="openLink(links.usage)"
              >
                <Icon name="external-link" :size="14" />
              </button>
              <button class="mini" type="button" :title="t('skills.refresh')" @click="loadBalance">
                <Icon name="reload" :size="14" />
              </button>
            </div>
          </div>

          <div class="row">
            <div class="text">
              <div class="label">{{ t("model.apiKey") }}</div>
              <div class="desc">{{ t("model.apiKeyDesc") }}</div>
            </div>
            <div class="secret">
              <input
                v-model="apiKeyDraft"
                class="input"
                type="password"
                autocomplete="off"
                :placeholder="engineReady ? '••••••••' : ''"
                @keydown.enter.prevent="saveKey"
              />
              <button class="mini" type="button" :title="t('model.save')" @click="saveKey">
                <Icon name="check" :size="14" />
              </button>
              <button class="mini" type="button" :title="t('model.clear')" @click="removeKey">
                <Icon name="x" :size="14" />
              </button>
            </div>
          </div>

          <div class="row">
            <div class="text">
              <div class="label">{{ t("model.baseUrl") }}</div>
              <div class="desc">{{ t("model.baseUrlDesc") }}</div>
            </div>
            <input v-model="settings.baseUrl" class="input" type="url" spellcheck="false" />
          </div>

          <div class="row">
            <div class="text">
              <div class="label">{{ t("model.model") }}</div>
              <!-- The name is what a person recognises; the id is what actually gets sent, so
                   both are on screen instead of only the one that looked wrong. -->
              <div class="desc">{{ modelIsKnown ? settings.model : t("model.modelDesc") }}</div>
            </div>
            <div class="model-field">
              <select v-if="modelIsKnown" v-model="settings.model" class="input">
                <option v-for="entry in modelChoices()" :key="entry.id" :value="entry.id">
                  {{ entry.name }}
                </option>
              </select>
              <input
                v-else
                v-model="settings.model"
                class="input"
                :title="t('model.customId')"
              />
              <button
                class="mini"
                type="button"
                :title="t('skills.refresh')"
                @click="loadModels"
              >
                <Icon name="reload" :size="14" />
              </button>
            </div>
          </div>

          <div v-if="effortsFor(settings.model).length" class="row">
            <div class="text">
              <div class="label">{{ t("model.effort") }}</div>
              <div class="desc">{{ t("model.effortDesc") }}</div>
            </div>
            <div class="model-field">
              <select v-model="settings.reasoningEffort" class="input">
                <option value="">{{ t("model.effortDefault") }}</option>
                <option v-for="level in effortsFor(settings.model)" :key="level" :value="level">
                  {{ level }}
                </option>
              </select>
            </div>
          </div>

          <div class="row">
            <div class="text">
              <div class="label">{{ t("permission.label") }}</div>
              <div class="desc">{{ t("permission.desc") }}</div>
            </div>
            <div class="segmented">
              <button
                v-for="mode in permissionModes"
                :key="mode.value"
                class="segment"
                :class="{ active: settings.permission === mode.value }"
                type="button"
                :title="t(mode.hint)"
                @click="choosePermission(mode.value)"
              >
                <Icon :name="mode.icon" :size="16" />
                <span>{{ t(mode.label) }}</span>
              </button>
            </div>
          </div>
          <p class="hint">{{ t("permission.hint") }}</p>
          <div v-if="trustedTools.length" class="row">
            <div class="text">
              <div class="label">{{ t("permission.trusted") }}</div>
              <div class="desc">{{ trustedTools.join(", ") }}</div>
            </div>
            <button class="segment" type="button" @click="clearToolGrants">
              {{ t("permission.revoke") }}
            </button>
          </div>
        </div>

        <p class="hint" :class="{ warn: secretBackend === 'file' }">{{ secretHint }}</p>
        <p v-if="modelsError" class="hint warn">{{ modelsError }}</p>
        <p v-if="balanceError" class="hint warn">{{ balanceError }}</p>
      </template>

      <template v-else-if="activeItem === 'memories'">
        <h1 class="title">{{ t("settings.items.memories") }}</h1>
        <MemoryPanel />
      </template>

      <template v-else-if="activeItem === 'sandbox'">
        <h1 class="title">{{ t("settings.items.sandbox") }}</h1>
        <SandboxPanel />
      </template>

      <template v-else-if="activeItem === 'partner'">
        <PartnerPanel />
      </template>

      <template v-else-if="activeItem === 'diagnostics'">
        <DiagnosticsPanel />
      </template>

      <template v-else-if="activeItem === 'about'">
        <AboutPanel />
      </template>

      <template v-else-if="activeItem === 'archived'">
        <div class="archive-head">
          <div>
            <h1 class="title">{{ t("archived.title") }}</h1>
            <p class="hint">{{ t("archived.hint") }}</p>
          </div>
          <button
            v-if="archived.length"
            class="mini-text danger"
            type="button"
            :disabled="archivedBusy"
            @click="archiveDeleteId = 'all'"
          >
            <Icon name="archive" :size="13" />
            {{ t("archived.deleteAll") }}
          </button>
        </div>

        <div v-if="archived.length" class="archive-tools">
          <label class="archive-search">
            <Icon name="search" :size="14" />
            <input
              v-model="archivedQuery"
              type="search"
              :placeholder="t('archived.search')"
              :aria-label="t('archived.search')"
            />
          </label>
          <select v-model="archivedProject" class="input archive-filter">
            <option value="">{{ t("archived.allProjects") }}</option>
            <option v-for="project in archivedProjects" :key="project.value" :value="project.value">
              {{ project.label }}
            </option>
          </select>
        </div>

        <p v-if="archived.length === 0" class="hint">{{ t("archived.empty") }}</p>
        <p v-else-if="filteredArchived.length === 0" class="hint">
          {{ t("archived.noMatch") }}
        </p>

        <div v-else class="card">
          <div v-for="session in filteredArchived" :key="session.id" class="row">
            <div class="text">
              <div class="label">{{ session.title || t("archived.untitled") }}</div>
              <div class="desc archive-meta">
                <Icon name="folder" :size="12" />
                <span>{{ projectLabel(session.workspace) }}</span>
                <span>·</span>
                <span>{{ archivedDate(session) }}</span>
              </div>
            </div>
            <button
              class="mini-text"
              type="button"
              :disabled="archivedBusy"
              @click="restore(session.id)"
            >
              {{ t("archived.restore") }}
            </button>
            <button
              class="mini-text danger"
              type="button"
              :disabled="archivedBusy"
              @click="archiveDeleteId = session.id"
            >
              {{ t("archived.delete") }}
            </button>
          </div>
        </div>
      </template>

      <div v-else class="placeholder">
        <Icon name="gear" :size="26" />
        <p>{{ t("common.comingSoon") }}</p>
      </div>
    </main>

    <ConfirmDialog
      :open="archiveDeleteId !== null"
      :title="archiveDeleteTitle"
      :message="archiveDeleteMessage"
      :confirm-label="t('archived.delete')"
      :cancel-label="t('common.cancel')"
      danger
      @confirm="deleteArchived"
      @cancel="archiveDeleteId = null"
    />

    <RudderDialog
      :open="rudderOpen"
      :name="partnerDisplayName"
      @confirm="confirmRudder"
      @cancel="cancelRudder"
    />

    <WorkspacePicker
      :open="workspacePickerOpen"
      :initial="settings.workspace"
      @choose="chooseWorkspace"
      @close="workspacePickerOpen = false"
    />
    <WorkspacePicker
      :open="projectsRootPickerOpen"
      :initial="settings.projectsRoot || settings.workspace"
      @choose="chooseProjectsRoot"
      @close="projectsRootPickerOpen = false"
    />
  </div>
</template>

<style scoped>
.settings {
  display: grid;
  grid-template-columns: 264px minmax(0, 1fr);
  grid-template-rows: minmax(0, 1fr);
  gap: 10px;
  height: 100%;
  padding: 10px;
  background: var(--bg-app);
}

.nav {
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
  padding: 10px;
}

.back {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 10px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text);
  text-align: left;
}

.back:hover {
  background: var(--bg-hover);
}

.search {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 8px 0 12px;
  padding: 0 10px;
  height: 30px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text-muted);
}

.search input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: transparent;
}

.nav-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.nav-group {
  margin-bottom: 14px;
}

.nav-group-title {
  padding: 4px 10px;
  color: var(--text-muted);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.5px;
  text-transform: uppercase;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 7px 10px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text);
  text-align: left;
}

.nav-item:hover {
  background: var(--bg-hover);
}

.nav-item.active {
  background: var(--bg-active);
  color: var(--text-strong);
}

/* `.card` below sets `overflow: hidden`; both classes sit on the same element, so the later
   rule won and the settings never scrolled. Two classes beat one. */
.settings .content {
  min-width: 0;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 28px 40px;
}

.title {
  margin: 0 0 22px;
  font-size: 20px;
  font-weight: 600;
  color: var(--text-strong);
}

.section {
  margin: 0 0 10px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-strong);
}

.mode-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 14px;
  margin-bottom: 28px;
}

.mode-card {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 16px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-panel);
  color: var(--text);
  text-align: left;
}

.mode-card.active {
  border-color: var(--accent);
}

.mode-text {
  flex: 1;
  min-width: 0;
}

.mode-title {
  color: var(--text-strong);
  font-size: 13px;
  font-weight: 600;
}

.mode-desc {
  margin-top: 3px;
  color: var(--text-muted);
  font-size: 12px;
}

.radio {
  flex: none;
  width: 16px;
  height: 16px;
  border: 1px solid var(--border-strong);
  border-radius: 50%;
}

.radio.on {
  border-color: var(--accent);
  box-shadow: inset 0 0 0 4px var(--accent);
}

.card {
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-panel);
  overflow: hidden;
}

.card > .row:first-child {
  border-top: none;
}

.row {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 14px 16px;
  border-top: 1px solid var(--border);
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
  margin-top: 2px;
  color: var(--text-muted);
  font-size: 12px;
}

.segmented {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 6px;
}

.segment {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 7px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text-muted);
  font-size: 12px;
  white-space: nowrap;
}

.segment.active {
  border-color: var(--accent);
  color: var(--text-strong);
}

.select,
.input {
  height: 30px;
  padding: 0 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text);
  font-size: 12px;
}

.input {
  width: 320px;
}

.workspace {
  display: flex;
  align-items: center;
  gap: 6px;
}

.mini {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text-muted);
}

.mini:hover {
  color: var(--text-strong);
}

.placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  height: 60%;
  color: var(--text-muted);
}

.value {
  color: var(--text);
  font-size: 13px;
}

.sizer {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 220px;
}

.sizer input {
  flex: 1;
  accent-color: var(--accent);
}

.sizer .value {
  width: 52px;
  text-align: right;
  color: var(--text-muted);
  font-size: 12px;
}

.hint {
  margin: 12px 0 0;
  color: var(--text-muted);
  font-size: 12px;
}

.hint.warn {
  color: var(--danger);
}

.archive-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.archive-tools {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin: 14px 0 12px;
}

.archive-search {
  display: flex;
  flex: 1;
  align-items: center;
  gap: 8px;
  min-width: 180px;
  padding: 0 10px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text-muted);
}

.archive-search input {
  flex: 1;
  min-width: 0;
  height: 30px;
  border: none;
  background: transparent;
  color: var(--text);
  font: inherit;
  font-size: 12px;
  outline: none;
}

.archive-filter {
  width: auto;
  min-width: 140px;
}

.archive-meta {
  display: flex;
  align-items: center;
  gap: 5px;
}

.mini-text {
  padding: 5px 12px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text);
  font-size: 12px;
}

.mini-text.danger {
  border-color: var(--danger);
  color: var(--danger);
}

.mini-text.danger:hover {
  border-color: var(--danger);
  color: var(--danger);
  background: var(--bg-hover);
}

.mini-text:disabled {
  opacity: 0.6;
}

.mini-text:hover {
  border-color: var(--accent);
  color: var(--text-strong);
}

.secret {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 1;
  min-width: 0;
  max-width: 360px;
}

.secret .input {
  flex: 1;
  min-width: 0;
  width: auto;
}

.model-field {
  display: flex;
  align-items: center;
  justify-content: end;
  gap: 6px;
  flex: 1;
  min-width: 0;
  max-width: 360px;
}

.model-field .input {
  flex: 1;
  min-width: 0;
  width: auto;
}
</style>
