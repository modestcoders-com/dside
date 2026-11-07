<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import ToggleRow from "./ToggleRow.vue";
import Icon from "../Icon.vue";
import { t } from "../../i18n";
import { defaultAllowedCommands, settings } from "../../stores/settings";

/**
 * Policies for the companion, not for the user's terminal. The allowlist is a policy:
 * program names, no paths, no shell redirection; it is deliberately not called a sandbox.
 *
 * The list is a row of chips rather than a text area: the names are short, there are dozens of
 * them, and a wall of text made it impossible to see what is allowed without reading every
 * line. Adding and removing an entry are one click each, and a typo is refused while typing.
 */
const allowlist = computed({
  get: () => settings.commandPolicy === "allowlist",
  set: (value: boolean) => {
    settings.commandPolicy = value ? "allowlist" : "all";
  },
});

const adding = ref(false);
const draft = ref("");
const addInput = ref<HTMLInputElement | null>(null);

/** The same shape the policy accepts: a bare program name, no path, no arguments. */
function validCommand(value: string): boolean {
  return /^[a-z0-9][a-z0-9._+-]*$/.test(value);
}

function removeCommand(command: string): void {
  settings.allowedCommands = settings.allowedCommands.filter((entry) => entry !== command);
}

async function beginAdd(): Promise<void> {
  if (!allowlist.value) return;
  adding.value = true;
  draft.value = "";
  await nextTick();
  addInput.value?.focus();
}

function cancelAdd(): void {
  adding.value = false;
  draft.value = "";
}

function commitAdd(): void {
  const clean = draft.value.trim().toLowerCase();
  if (!clean) {
    cancelAdd();
    return;
  }
  if (!validCommand(clean)) return;
  if (!settings.allowedCommands.includes(clean)) {
    settings.allowedCommands = [...settings.allowedCommands, clean];
  }
  cancelAdd();
}

function resetCommands(): void {
  settings.allowedCommands = [...defaultAllowedCommands];
  cancelAdd();
}
</script>

<template>
  <div class="sandbox">
    <p class="hint">{{ t("sandbox.hint") }}</p>

    <div class="card">
      <ToggleRow
        :model-value="allowlist"
        :label="t('sandbox.allowlist')"
        :description="t('sandbox.allowlistDesc')"
        @update:model-value="allowlist = $event"
      />

      <div class="commands" :class="{ disabled: !allowlist }">
        <div class="commands-head">
          <span class="label">{{ t("sandbox.commands") }}</span>
          <button class="mini-text" type="button" @click="resetCommands">
            {{ t("sandbox.reset") }}
          </button>
        </div>

        <div class="command-grid">
          <span v-for="command in settings.allowedCommands" :key="command" class="command">
            <span class="command-name" :title="command">{{ command }}</span>
            <button
              class="command-remove"
              type="button"
              :title="t('sandbox.removeCommand', { command })"
              :disabled="!allowlist"
              @click="removeCommand(command)"
            >
              <Icon name="x" :size="10" />
            </button>
          </span>

          <span v-if="adding" class="command command-adding">
            <input
              ref="addInput"
              v-model="draft"
              :placeholder="t('sandbox.addPlaceholder')"
              spellcheck="false"
              @keydown.enter.prevent="commitAdd"
              @keydown.esc.prevent="cancelAdd"
              @blur="cancelAdd"
            />
          </span>
          <button
            v-else
            class="command command-add"
            type="button"
            :disabled="!allowlist"
            :title="t('sandbox.addCommand')"
            @click="beginAdd"
          >
            <Icon name="plus" :size="11" />
            <span>{{ t("sandbox.add") }}</span>
          </button>
        </div>

        <p v-if="adding && draft && !validCommand(draft.trim().toLowerCase())" class="invalid">
          {{ t("sandbox.invalidCommand") }}
        </p>
        <p class="hint">{{ t("sandbox.commandsHint") }}</p>
      </div>

      <ToggleRow
        :model-value="settings.workspaceOnly"
        :label="t('sandbox.workspace')"
        :description="t('sandbox.workspaceDesc')"
        @update:model-value="settings.workspaceOnly = $event"
      />
    </div>

    <p class="note">{{ t("sandbox.note") }}</p>
  </div>
</template>

<style scoped>
.sandbox {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.hint {
  margin: 0;
  color: var(--text-muted);
  font-size: 12px;
  line-height: 1.5;
}

.card {
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-panel);
}

.commands {
  padding: 14px 16px;
  border-top: 1px solid var(--border);
}

.commands.disabled {
  opacity: 0.55;
}

.commands-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 8px;
}

.label {
  color: var(--text-strong);
  font-size: 13px;
}

/* A grid of equal cells: the names line up, and a long one cannot push the row's edge. */
.command-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 8px;
}

.command {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  box-sizing: border-box;
  width: 13ch;
  min-height: 26px;
  padding: 3px 5px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text);
  font-family: var(--mono);
  font-size: 10.5px;
}

.command-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.command-remove {
  display: grid;
  flex: none;
  place-items: center;
  width: 14px;
  height: 14px;
  border: 0;
  border-radius: 3px;
  background: transparent;
  color: var(--text-muted);
}

.command-remove:hover:not(:disabled) {
  background: var(--bg-hover);
  color: var(--danger);
}

.command-add {
  gap: 4px;
  justify-content: center;
  border-style: dashed;
  color: var(--text-muted);
}

.command-add:hover:not(:disabled) {
  border-color: var(--accent);
  color: var(--accent);
}

.command-add:disabled,
.command-remove:disabled {
  cursor: default;
  opacity: 0.45;
}

.command-adding {
  padding: 0;
}

.command-adding input {
  width: 100%;
  height: 24px;
  padding: 0 5px;
  border: 0;
  outline: none;
  background: transparent;
  color: var(--text);
  font-family: var(--mono);
  font-size: 10.5px;
}

.invalid {
  margin: 6px 0 0;
  color: var(--danger);
  font-size: 10.5px;
}

.commands .hint {
  margin-top: 6px;
}

.note {
  margin: 0;
  padding: 9px 11px;
  border-left: 2px solid var(--accent);
  background: var(--bg-panel);
  color: var(--text-muted);
  font-size: 12px;
  line-height: 1.5;
}
</style>
