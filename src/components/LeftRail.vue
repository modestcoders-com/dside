<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import Icon from "./Icon.vue";
import DeepSeekWhale from "./DeepSeekWhale.vue";
import WorkspacePicker from "./WorkspacePicker.vue";
import { t } from "../i18n";
import { deepSeekMark } from "./deepseek-mark";
import { everydayMode } from "../engine/mode";
import { links } from "../engine/links";
import type { Session, SessionGroup } from "../types";
import { activeSessionId, sessions } from "../stores/sessions";
import { settings } from "../stores/settings";
import { openSearch, openSettings, railCollapsed } from "../stores/ui";
import { openTab } from "../stores/workArea";
import { groupByProject } from "../lib/projects";
import {
  activeProject,
  activateProject,
  loadProjectFolders,
  projectError,
  projectFolders,
  switchProject,
} from "../composables/useProjects";
import {
  clearApiKey,
  createSession,
  engineReady,
  loadSessions,
  openSession,
} from "../composables/useEngine";

const accountOpen = ref(false);
const confirmSignOut = ref(false);
const systemName = ref("User");
const displayName = computed(() => settings.displayName.trim() || systemName.value);

onMounted(async () => {
  if (settings.displayName.trim()) return;
  try {
    systemName.value = await invoke<string>("default_display_name");
  } catch {
    // keep the fallback
  }
});

async function openLink(url: string) {
  accountOpen.value = false;
  try {
    // Our own window keeps focus under our control — the user's browser does not.
    if (/^https?:/i.test(url)) await invoke("open_browser_window", { url });
    else await openUrl(url);
  } catch {
    // nothing sensible to do if even that fails
  }
}

async function signOut() {
  // First click only asks. The key is a credential — losing it by accident is rude.
  if (!confirmSignOut.value) {
    confirmSignOut.value = true;
    return;
  }
  confirmSignOut.value = false;
  accountOpen.value = false;
  await clearApiKey();
}

watch(accountOpen, (open) => {
  if (!open) confirmSignOut.value = false;
});

function signIn() {
  accountOpen.value = false;
  openSettings("usage");
}
const menuSession = ref<Session | null>(null);
const menuX = ref(0);
const menuY = ref(0);
const renamingId = ref<string | null>(null);
const renameDraft = ref("");

const vFocus = {
  mounted: (element: HTMLInputElement) => element.focus(),
};

function openMenu(event: MouseEvent, session: Session) {
  event.preventDefault();
  menuX.value = event.clientX;
  menuY.value = event.clientY;
  menuSession.value = session;
}

function startRename(session: Session) {
  menuSession.value = null;
  renamingId.value = session.id;
  renameDraft.value = session.title;
}

async function commitRename() {
  const id = renamingId.value;
  const title = renameDraft.value.trim();
  renamingId.value = null;
  if (!id || !title) return;
  await invoke("session_update", { id, title });
  await loadSessions();
}

async function togglePin(session: Session) {
  menuSession.value = null;
  await invoke("session_update", {
    id: session.id,
    group: session.group === "pinned" ? "project" : "pinned",
  });
  await loadSessions();
}

/** Hides the session from the rail. Nothing is deleted — see the archive in Settings. */
async function archiveSession(session: Session) {
  menuSession.value = null;
  await invoke("session_archive", { id: session.id, archived: true });
  await loadSessions();
}

const ALL_PRIMARY_ITEMS = [
  { icon: "search", key: "nav.search", soon: false },
  { icon: "grid", key: "nav.skills", soon: false },
  { icon: "clock", key: "nav.scheduled", soon: false },
] as const;

/** Everyday mode leaves out the skills and schedules entries. */
const primaryItems = computed(() =>
  everydayMode.value
    ? ALL_PRIMARY_ITEMS.filter((item) => item.key === "nav.search")
    : ALL_PRIMARY_ITEMS,
);

const sectionOrder: SessionGroup[] = ["pinned", "scheduled", "project", "team"];

const sectionKeys: Record<SessionGroup, "sections.pinned" | "sections.scheduled" | "sections.projects" | "sections.agentTeam"> = {
  pinned: "sections.pinned",
  scheduled: "sections.scheduled",
  project: "sections.projects",
  team: "sections.agentTeam",
};

function groupItems(group: SessionGroup) {
  return sessions.value.filter((session) => session.group === group);
}

/** The "Projekte" section, split by the folder each thread was started in. */
const projectBuckets = computed(() =>
  groupByProject(groupItems("project"), t("projects.none")),
);

/**
 * One menu, two uses: choose the project to work in, or move a thread into one. Keeping it a
 * single panel means there is exactly one list of projects in the app.
 */
const projectMenuOpen = ref(false);
const assignTarget = ref<Session | null>(null);
const pickerOpen = ref(false);

function openProjectMenu(): void {
  assignTarget.value = null;
  projectMenuOpen.value = !projectMenuOpen.value;
}

function openAssignMenu(session: Session): void {
  menuSession.value = null;
  assignTarget.value = session;
  projectMenuOpen.value = true;
}

function closeProjectMenu(): void {
  projectMenuOpen.value = false;
  assignTarget.value = null;
}

async function chooseProject(path: string): Promise<void> {
  const target = assignTarget.value;
  closeProjectMenu();
  if (!target) {
    await switchProject(path);
    return;
  }
  await invoke("session_set_workspace", { id: target.id, workspace: path });
  await loadSessions();
  // Moving the thread that is open moves the tools with it: otherwise the next message in
  // this thread would file its work under the folder it was moved *away* from.
  if (target.id === activeSessionId.value) await activateProject(path);
}

function chooseFolder(path: string): void {
  pickerOpen.value = false;
  void chooseProject(path);
}

/** A thread in the active project — the same thing "Neue Aufgabe" did. */
function newThread(): void {
  void createSession();
}

// The list is read again when the base changes, so the dropdown never shows another folder's
// children after the user typed a new base.
watch(
  () => settings.projectsRoot,
  () => void loadProjectFolders(),
  { immediate: true },
);

function onPrimary(key: string) {
  if (key === "nav.search") {
    openSearch();
    return;
  }
  if (key === "nav.skills") {
    openTab("skills");
    return;
  }
  if (key === "nav.scheduled") {
    openTab("schedules");
    return;
  }
}

function accountSettings() {
  accountOpen.value = false;
  openSettings();
}
</script>

<template>
  <aside class="rail" :class="{ collapsed: railCollapsed }">
    <div v-if="menuSession" class="menu-backdrop" @click="menuSession = null" @contextmenu.prevent="menuSession = null"></div>
    <div
      v-if="menuSession"
      class="session-menu"
      :style="{ left: `${menuX}px`, top: `${menuY}px` }"
    >
      <button class="menu-item" type="button" @click="togglePin(menuSession)">
        <Icon name="pin" :size="14" />
        <span>{{ menuSession.group === "pinned" ? t("session.unpin") : t("session.pin") }}</span>
      </button>
      <button class="menu-item" type="button" @click="openAssignMenu(menuSession)">
        <Icon name="folder" :size="14" />
        <span>{{ t("projects.moveTo") }}</span>
      </button>
      <button class="menu-item" type="button" @click="startRename(menuSession)">
        <Icon name="pencil" :size="14" />
        <span>{{ t("session.rename") }}</span>
      </button>
      <button class="menu-item" type="button" @click="archiveSession(menuSession)">
        <Icon name="archive" :size="14" />
        <span>{{ t("session.archive") }}</span>
      </button>
    </div>
    <!-- The project row replaces "Neue Aufgabe": it says which folder the tools are pointing
         at and starts the next thread in it. Two states, one place. -->
    <div class="project-row">
      <button
        class="project-pick"
        :class="{ open: projectMenuOpen }"
        type="button"
        :title="settings.workspace || t('projects.noneHint')"
        @click="openProjectMenu"
      >
        <Icon name="folder" :size="14" />
        <span class="project-name">{{ activeProject || t("projects.none") }}</span>
        <Icon name="chevron-down" :size="12" class="project-chevron" />
      </button>
      <button
        class="project-new"
        type="button"
        :title="t('projects.newThread', { project: activeProject || t('projects.none') })"
        @click="newThread"
      >
        <Icon name="plus" :size="14" />
      </button>
    </div>

    <template v-if="projectMenuOpen">
      <div class="menu-backdrop" @click="closeProjectMenu"></div>
      <div class="project-menu">
        <div class="menu-head">
          <span class="menu-name">
            {{
              assignTarget
                ? t("projects.assignTo", { title: assignTarget.title })
                : t("projects.choose")
            }}
          </span>
        </div>
        <div class="project-list">
          <button
            v-for="folder in projectFolders"
            :key="folder.path"
            class="menu-item"
            type="button"
            :title="folder.path"
            @click="chooseProject(folder.path)"
          >
            <Icon
              :name="folder.path === settings.workspace ? 'check' : 'folder'"
              :size="14"
            />
            <span class="session-title">{{ folder.name }}</span>
          </button>
          <p v-if="projectError" class="menu-note warn">{{ projectError }}</p>
          <p v-else-if="!projectFolders.length" class="menu-note">
            {{ t("projects.noBase") }}
          </p>
        </div>
        <div class="menu-separator"></div>
        <button class="menu-item" type="button" @click="chooseProject('')">
          <Icon name="x" :size="14" />
          <span>{{ t("projects.none") }}</span>
        </button>
        <button class="menu-item" type="button" @click="pickerOpen = true">
          <Icon name="folder" :size="14" />
          <span>{{ t("projects.openFolder") }}</span>
        </button>
      </div>
    </template>

    <WorkspacePicker
      :open="pickerOpen"
      :initial="settings.workspace || settings.projectsRoot"
      @choose="chooseFolder"
      @close="pickerOpen = false"
    />

    <nav class="rail-nav">
      <button
        v-for="item in primaryItems"
        :key="item.key"
        class="nav-item"
        type="button"
        @click="onPrimary(item.key)"
      >
        <Icon :name="item.icon" />
        <span>{{ t(item.key) }}</span>
        <span v-if="item.soon" class="soon">{{ t("nav.soon") }}</span>
      </button>
    </nav>

    <div class="rail-scroll">
      <section v-for="group in sectionOrder" :key="group" class="rail-section">
        <div class="section-head">{{ t(sectionKeys[group]) }}</div>

        <!-- Threads grouped by the project they belong to; the folder is the header. -->
        <template v-if="group === 'project'">
          <p v-if="projectBuckets.length === 0" class="empty">{{ t("sections.noTasks") }}</p>
          <div v-for="bucket in projectBuckets" :key="bucket.workspace || '__none'">
            <div
              v-if="bucket.workspace || projectBuckets.length > 1"
              class="project-head"
              :title="bucket.workspace || t('projects.noneHint')"
            >
              <Icon name="folder" :size="12" />
              <span class="session-title">{{ bucket.label }}</span>
            </div>
            <button
              v-for="session in bucket.threads"
              :key="session.id"
              class="session"
              :class="{ active: session.id === activeSessionId }"
              type="button"
              @click="openSession(session.id)"
              @contextmenu.prevent="openMenu($event, session)"
            >
              <Icon name="doc" :size="14" />
              <input
                v-if="renamingId === session.id"
                v-model="renameDraft"
                v-focus
                class="rename"
                @click.stop
                @keydown.enter.prevent="commitRename"
                @keydown.esc.prevent="renamingId = null"
                @blur="commitRename"
              />
              <span v-else class="session-title">{{ session.title }}</span>
            </button>
          </div>
        </template>

        <template v-else>
          <p v-if="groupItems(group).length === 0" class="empty">{{ t("sections.noTasks") }}</p>
          <button
            v-for="session in groupItems(group)"
            :key="session.id"
            class="session"
            :class="{ active: session.id === activeSessionId }"
            type="button"
            @click="openSession(session.id)"
            @contextmenu.prevent="openMenu($event, session)"
          >
            <Icon name="doc" :size="14" />
            <input
              v-if="renamingId === session.id"
              v-model="renameDraft"
              v-focus
              class="rename"
              @click.stop
              @keydown.enter.prevent="commitRename"
              @keydown.esc.prevent="renamingId = null"
              @blur="commitRename"
            />
            <span v-else class="session-title">{{ session.title }}</span>
          </button>
        </template>
      </section>
    </div>

    <div class="rail-foot">
      <button class="account" type="button" @click.stop="accountOpen = !accountOpen">
        <DeepSeekWhale class="brand-mark" :height="26" :label="t('app.name')" />
        <span class="ds-mark" v-html="deepSeekMark"></span>
        <Icon name="chevron-down" :size="14" class="account-chevron" />
      </button>

      <div v-if="accountOpen" class="backdrop" @click="accountOpen = false"></div>
      <div v-if="accountOpen" class="account-menu">
        <div class="menu-head">
          <span class="menu-name">{{ displayName }}</span>
        </div>
        <button class="menu-item" type="button" @click="accountSettings">
          <Icon name="gear" :size="14" /><span>{{ t("account.settings") }}</span>
        </button>
        <button class="menu-item" type="button" @click="openLink(links.provider)">
          <Icon name="user" :size="14" /><span>{{ t("account.account") }}</span>
          <Icon name="external-link" :size="14" class="trail" />
        </button>
        <button class="menu-item" type="button" @click="openLink(links.usage)">
          <Icon name="grid" :size="14" /><span>{{ t("account.usage") }}</span>
          <Icon name="external-link" :size="14" class="trail" />
        </button>
        <button class="menu-item" type="button" @click="openLink(links.issues)">
          <Icon name="send" :size="14" /><span>{{ t("account.feedback") }}</span>
          <Icon name="external-link" :size="14" class="trail" />
        </button>
        <button class="menu-item" type="button" @click="openLink(links.mail)">
          <Icon name="doc" :size="14" /><span>{{ t("account.contact") }}</span>
          <Icon name="external-link" :size="14" class="trail" />
        </button>
        <button class="menu-item" type="button" @click="openLink(links.home)">
          <Icon name="globe" :size="14" /><span>{{ t("account.learnMore") }}</span>
          <Icon name="external-link" :size="14" class="trail" />
        </button>
        <button
          v-if="engineReady"
          class="menu-item"
          :class="{ danger: confirmSignOut }"
          type="button"
          @click="signOut"
        >
          <Icon :name="confirmSignOut ? 'x' : 'external-link'" :size="14" />
          <span>{{ confirmSignOut ? t("model.clear") : t("account.logout") }}</span>
        </button>
        <button v-else class="menu-item accent" type="button" @click="signIn">
          <Icon name="sparkle" :size="14" /><span>{{ t("account.signIn") }}</span>
        </button>
      </div>
    </div>
  </aside>
</template>

<style scoped>
.rail {
  position: relative;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: visible;
}

.brand-mark {
  height: 24px;
  width: auto;
  object-fit: contain;
}

.ds-mark {
  display: inline-flex;
  color: var(--text-strong);
  margin: 0 auto;
}

.ds-mark :deep(svg) {
  display: block;
  height: 15px;
  width: auto;
}

.rail-nav {
  display: flex;
  flex-direction: column;
  padding: 14px 8px 8px;
}

/* The project row: a folder picker where "Neue Aufgabe" used to be. The name takes the
   space it can get, the chevron and the "+" stay fixed so a long path never pushes them out. */
.project-row {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 14px 8px 0;
}

.project-pick,
.project-new {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 30px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-panel);
  color: var(--text-strong);
  font-size: 12px;
}

.project-pick {
  flex: 1;
  min-width: 0;
  padding: 0 6px 0 9px;
  text-align: left;
}

.project-pick:hover,
.project-pick.open {
  background: var(--bg-hover);
}

.project-pick.open {
  border-color: var(--accent);
}

.project-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project-chevron {
  flex: none;
  color: var(--text-muted);
}

.project-new {
  flex: none;
  justify-content: center;
  width: 30px;
  padding: 0;
  color: var(--text-muted);
}

.project-new:hover {
  background: var(--bg-hover);
  color: var(--text-strong);
}

.project-menu {
  position: absolute;
  top: 44px;
  right: 8px;
  left: 8px;
  z-index: 43;
  min-width: 190px;
  padding: 5px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  background: var(--bg-elevated);
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.55);
}

.project-menu .menu-head {
  padding: 5px 8px 6px;
  color: var(--text-muted);
  font-size: 11px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project-list {
  max-height: 240px;
  overflow-y: auto;
}

.menu-note {
  margin: 4px 8px 6px;
  color: var(--text-muted);
  font-size: 11px;
  line-height: 1.4;
}

.menu-note.warn {
  color: var(--danger);
}

.menu-separator {
  height: 1px;
  margin: 5px 4px;
  background: var(--border);
}

/* A folder header inside "Projekte": one level of nesting, no extra chrome. */
.project-head {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 7px 10px 2px;
  color: var(--text-muted);
  font-size: 11px;
}

.project-head .session-title {
  min-width: 0;
}

.nav-item,
.session,
.menu-item {
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
  white-space: nowrap;
}

/* Long titles are cut with an ellipsis instead of running out of the rail. */
.session-title {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.nav-item:hover,
.session:hover,
.menu-item:hover {
  background: var(--bg-hover);
}

.soon {
  margin-left: auto;
  padding: 0 6px;
  border: 1px solid var(--border-strong);
  border-radius: 999px;
  color: var(--text-muted);
  font-size: 10px;
  line-height: 15px;
}

.rename {
  flex: 1;
  min-width: 0;
  height: 20px;
  padding: 0 4px;
  border: 1px solid var(--accent);
  border-radius: 3px;
  background: var(--bg-input);
  color: var(--text-strong);
  font-size: 12px;
}

.menu-backdrop {
  position: fixed;
  inset: 0;
  z-index: 41;
}

.session-menu {
  position: fixed;
  z-index: 42;
  min-width: 180px;
  padding: 6px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  background: var(--bg-elevated);
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.55);
}

.session.active {
  background: var(--bg-active);
  color: var(--text-strong);
}

.rail-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 4px 8px 12px;
}

.rail-section {
  margin-top: 12px;
}

.section-head {
  padding: 4px 10px;
  color: var(--text-muted);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.6px;
  text-transform: uppercase;
}

.empty {
  margin: 2px 0 0;
  padding: 4px 10px;
  color: var(--text-muted);
  font-size: 12px;
}

.rail-foot {
  position: relative;
  padding: 8px;
  border-top: 1px solid var(--border);
}

.account {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 6px 10px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-strong);
}

.account:hover {
  background: var(--bg-hover);
}

.account-chevron {
  margin-left: auto;
  color: var(--text-muted);
}

.backdrop {
  position: fixed;
  inset: 0;
  z-index: 39;
}

.account-menu {
  position: absolute;
  bottom: 52px;
  left: 8px;
  z-index: 40;
  width: min(260px, calc(100vw - 24px));
  padding: 6px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  background: var(--bg-elevated);
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.55);
}

.menu-head {
  display: flex;
  align-items: baseline;
  gap: 8px;
  padding: 6px 10px 10px;
}

.menu-name {
  color: var(--text-strong);
  font-size: 13px;
  font-weight: 600;
}

.menu-provider {
  color: var(--text-muted);
  font-size: 11px;
}

.menu-item.accent {
  color: var(--accent);
}

.menu-item.danger {
  color: var(--danger);
}

.trail {
  margin-left: auto;
  color: var(--text-muted);
}
</style>
