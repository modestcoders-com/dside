<script setup lang="ts">
import { ref } from "vue";
import Icon from "./Icon.vue";
import ToolbarActions from "./ToolbarActions.vue";
import { t } from "../i18n";
import { activeSession, sessions } from "../stores/sessions";
import { toggleRail } from "../stores/ui";
import { openSession } from "../composables/useEngine";
import { openUpdatePage, updateAvailable } from "../composables/useUpdateCheck";

/**
 * The title is also the session switcher. The chevron used to be decoration: it promised a
 * menu that did not exist. With the rail collapsed there was no way to change session at all,
 * so this is where the list belongs.
 */
const open = ref(false);

async function pick(id: string): Promise<void> {
  open.value = false;
  if (id === activeSession.value?.id) return;
  await openSession(id);
}
</script>

<template>
  <header class="titlebar">
    <div class="side left">
      <button class="ghost" type="button" :title="t('nav.collapseSidebar')" @click="toggleRail">
        <Icon name="panel-left" :size="18" />
      </button>
      <!-- Only when there is something to get: a permanent "up to date" label is noise. -->
      <button
        v-if="updateAvailable"
        class="update"
        type="button"
        :title="
          t('update.hint', {
            version: updateAvailable.latest,
            current: updateAvailable.current,
          })
        "
        @click="openUpdatePage"
      >
        <Icon name="arrow-up" :size="12" />
        <span>{{ t("update.label", { version: updateAvailable.latest }) }}</span>
      </button>
    </div>

    <div class="center">
      <button class="title" type="button" :title="t('titlebar.switch')" @click="open = !open">
        <span class="session">{{ activeSession?.title ?? "" }}</span>
        <Icon :name="open ? 'chevron-up' : 'chevron-down'" :size="14" />
      </button>

      <div v-if="open" class="backdrop" @click="open = false"></div>
      <div v-if="open" class="picker card">
        <button
          v-for="session in sessions"
          :key="session.id"
          class="pick"
          :class="{ active: session.id === activeSession?.id }"
          type="button"
          @click="pick(session.id)"
        >
          <span class="pick-title">{{ session.title || t("session.titlePlaceholder") }}</span>
          <Icon v-if="session.id === activeSession?.id" name="check" :size="12" />
        </button>
        <p v-if="sessions.length === 0" class="empty">{{ t("session.empty") }}</p>
      </div>
    </div>

    <div class="side right">
      <ToolbarActions />
    </div>
  </header>
</template>

<style scoped>
.titlebar {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  height: 44px;
  padding: 0 14px;
  background: transparent;
}

.side {
  display: flex;
  align-items: center;
  gap: 6px;
}

/* Small and green, and only while there is something to get. */
.update {
  display: flex;
  align-items: center;
  gap: 4px;
  height: 24px;
  padding: 0 9px;
  border: 1px solid var(--update);
  border-radius: 999px;
  background: color-mix(in srgb, var(--update) 16%, transparent);
  color: var(--update);
  font-size: 11.5px;
  font-weight: 600;
  white-space: nowrap;
  cursor: pointer;
}

.update:hover {
  background: color-mix(in srgb, var(--update) 28%, transparent);
}

.right {
  justify-content: flex-end;
}

.center {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-strong);
  font-size: 14px;
  font-weight: 600;
  white-space: nowrap;
}

.title {
  display: flex;
  align-items: center;
  gap: 6px;
  max-width: min(420px, 40vw);
  padding: 5px 10px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: inherit;
  font: inherit;
  cursor: pointer;
}

.title:hover {
  background: var(--bg-hover);
}

.session {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.backdrop {
  position: fixed;
  inset: 0;
  z-index: 20;
}

.picker {
  position: absolute;
  top: 38px;
  left: 0;
  right: 0;
  z-index: 21;
  max-height: 340px;
  overflow-y: auto;
  padding: 6px;
  text-align: left;
}

.pick {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  width: 100%;
  padding: 7px 9px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text);
  font-size: 12.5px;
  font-weight: 400;
  text-align: left;
  cursor: pointer;
}

.pick:hover {
  background: var(--bg-hover);
}

.pick.active {
  color: var(--text-strong);
}

.pick-title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.empty {
  margin: 6px 9px;
  color: var(--text-muted);
  font-size: 12px;
  font-weight: 400;
}

.ghost {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-muted);
}

.ghost:hover {
  background: var(--bg-hover);
  color: var(--text-strong);
}
</style>
