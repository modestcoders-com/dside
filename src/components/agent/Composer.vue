<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import Icon from "../Icon.vue";
import ContextRing from "./ContextRing.vue";
import RudderDialog from "../RudderDialog.vue";
import { t } from "../../i18n";
import { settings } from "../../stores/settings";
import { openSettings } from "../../stores/ui";
import { treeError } from "../../composables/useWorkspace";
import { partnerDisplayName } from "../../stores/partner";
import { modelChoices, modelLabel } from "../../stores/models";
import { activeSessionId } from "../../stores/sessions";
import {
  abortTurn,
  engineReady,
  loadHistory,
  running,
  sendPrompt,
} from "../../composables/useEngine";

import { draftFor, forgetDraft, rememberDraft } from "../../lib/drafts";

const text = ref("");

/**
 * The field keeps what was left in it — per session, across a restart. A sentence started
 * under one task does not belong in the next one, so the draft follows the session id
 * rather than the component. Only the field's text is kept: attachments keep their own
 * lifecycle and must not be resumed behind the user's back.
 */
let draftTimer: number | undefined;

function saveDraftNow(): void {
  if (draftTimer !== undefined) {
    window.clearTimeout(draftTimer);
    draftTimer = undefined;
  }
  rememberDraft(activeSessionId.value, text.value);
}

function cancelDraftSave(): void {
  if (draftTimer !== undefined) {
    window.clearTimeout(draftTimer);
    draftTimer = undefined;
  }
}

// The composer is created once and outlives the session it belongs to.
text.value = draftFor(activeSessionId.value);

watch(text, () => {
  cancelDraftSave();
  draftTimer = window.setTimeout(saveDraftNow, 300);
});

watch(activeSessionId, (next, previous) => {
  // File the text under the session being left *before* the field changes under it.
  cancelDraftSave();
  rememberDraft(previous ?? "", text.value);
  text.value = draftFor(next ?? "");
  void nextTick(() => autoGrow());
});

/**
 * The composer is where the next sentence starts. A chat window that opens with the
 * cursor somewhere in the chrome makes the user click before typing; keeping it here
 * also means the keyboard shortcuts for the field work immediately.
 *
 * The mount focus alone is not enough: the main window is still hidden while the splash
 * runs, and a hidden webview drops the focus. The window's own focus event fires when it
 * is finally shown, so the field is ready at the first keystroke.
 */
let removeFocusListener: (() => void) | null = null;
let removeShownListener: (() => void) | null = null;
let focusFallback: number | undefined;

/** Never take the cursor away from a field someone is already typing in. */
function composerFocusIsSafe(): boolean {
  const active = document.activeElement as HTMLElement | null;
  return !(
    active &&
    (active.tagName === "INPUT" ||
      active.tagName === "TEXTAREA" ||
      active.tagName === "SELECT" ||
      active.isContentEditable)
  );
}

function focusComposerIfIdle(): void {
  if (!composerFocusIsSafe()) return;
  box.value?.focus();
  // A draft restored at start was measured while the window was hidden; now there is a box.
  autoGrow();
}

onMounted(() => {
  // A restored draft can be several lines long; the field grows once before the first key.
  void nextTick(() => {
    autoGrow();
    box.value?.focus();
  });

  // Register both listeners independently: awaiting one must not delay the other. The
  // window-focus listener is the normal path; the splash's `main-shown` is the reliable
  // signal that the hidden main window has appeared.
  void listen("main-shown", () => focusComposerIfIdle())
    .then((unlisten) => {
      removeShownListener = unlisten;
    })
    .catch(() => undefined);
  void getCurrentWindow()
    .onFocusChanged(({ payload: focused }) => {
      if (focused) focusComposerIfIdle();
    })
    .then((unlisten) => {
      removeFocusListener = unlisten;
    })
    .catch(() => undefined);

  // The window is hidden for the splash's full run. A single late focus catches the case
  // where neither event arrives on this platform; it never steals from a field.
  focusFallback = window.setTimeout(() => focusComposerIfIdle(), 9000);
});

onBeforeUnmount(() => {
  // A reload or a shutdown must not eat the last 300 ms of typing.
  saveDraftNow();
  if (focusFallback !== undefined) window.clearTimeout(focusFallback);
  removeFocusListener?.();
  removeFocusListener = null;
  removeShownListener?.();
  removeShownListener = null;
});

/** Lets the panel put the cursor back after a turn without stealing it from another field. */
function focusComposer(): void {
  box.value?.focus();
}

/**
 * Puts text into the field from outside — the chat panel uses it when an answer card expires:
 * a sentence someone typed belongs in the next message, not in the bin.
 */
function setDraft(value: string): void {
  text.value = value;
  void nextTick(() => {
    autoGrow();
    box.value?.focus();
  });
}

defineExpose({ focus: focusComposer, setDraft });

interface PendingAttachment {
  path: string;
  name: string;
  mime: string;
  /** A small copy for the chat, so a conversation does not reload full pictures. */
  preview: string;
}

interface ScreenCapture {
  name: string;
  mime: string;
  dataBase64: string;
}

const attachments = ref<PendingAttachment[]>([]);
const fileInput = ref<HTMLInputElement | null>(null);
const capturing = ref(false);
const rudderOpen = ref(false);

/** Scales the picture down and re-encodes it: the chat shows this, the model gets the original. */
function resizeImage(source: string, maxSide: number, quality: number): Promise<string> {
  return new Promise<string>((resolve) => {
    const image = new Image();
    image.onload = () => {
      const scale = Math.min(1, maxSide / Math.max(image.width, image.height));
      const canvas = document.createElement("canvas");
      canvas.width = Math.max(1, Math.round(image.width * scale));
      canvas.height = Math.max(1, Math.round(image.height * scale));
      const context = canvas.getContext("2d");
      if (!context) return resolve(source);
      context.drawImage(image, 0, 0, canvas.width, canvas.height);
      resolve(canvas.toDataURL("image/jpeg", quality));
    };
    image.onerror = () => resolve(source);
    image.src = source;
  });
}

function makePreview(source: string): Promise<string> {
  return resizeImage(source, 480, 0.72);
}

async function addImages(files: File[]): Promise<void> {
  for (const file of files) {
    if (!file.type.startsWith("image/")) continue;
    const dataUrl = await new Promise<string>((resolve) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result ?? ""));
      reader.readAsDataURL(file);
    });
    if (!dataUrl.includes(",")) continue;
    const preview = await makePreview(dataUrl);
    try {
      const path = await invoke<string>("attachment_save", {
        name: file.name || "image",
        mime: file.type,
        dataBase64: dataUrl.slice(dataUrl.indexOf(",") + 1),
      });
      attachments.value.push({
        path,
        name: file.name || "image",
        mime: file.type,
        preview,
      });
    } catch {
      // A file that could not be stored is not worth breaking the message over.
    }
  }
}

function onPaste(event: ClipboardEvent): void {
  const files = Array.from(event.clipboardData?.files ?? []);
  if (files.length === 0) return;
  event.preventDefault();
  void addImages(files);
}

function onFiles(event: Event): void {
  const input = event.target as HTMLInputElement;
  void addImages(Array.from(input.files ?? []));
  input.value = "";
}

/** A short-lived line under the field: what compression just did, or that there was nothing. */
const note = ref<string | null>(null);

function onCompacted(folded: number | null): void {
  note.value = folded ? t("composer.compacted", { count: folded }) : t("composer.compactNothing");
  // The summary that just replaced the older turns is the "where we were" card.
  void loadHistory(activeSessionId.value);
  window.setTimeout(() => {
    note.value = null;
  }, 6000);
}

function removeAttachment(index: number): void {
  attachments.value.splice(index, 1);
}

/**
 * The companion's eyes. The screen is captured by Rust (PowerShell / screencapture / the
 * usual Linux tools), saved next to the app data and then attached like a pasted picture.
 * The model already receives attachments through the normal image path.
 */
async function captureScreen(): Promise<void> {
  if (capturing.value) return;
  capturing.value = true;
  note.value = null;
  try {
    const shot = await invoke<ScreenCapture>("capture_screen");
    const source = `data:${shot.mime};base64,${shot.dataBase64}`;
    const preview = await makePreview(source);
    // A full 4K PNG is several megabytes; the model gets a readable JPEG, the chat a thumb.
    const modelImage = await resizeImage(source, 2000, 0.86);
    const name = shot.name.replace(/\.[^.]+$/, "") + ".jpg";
    const path = await invoke<string>("attachment_save", {
      name,
      mime: "image/jpeg",
      dataBase64: modelImage.slice(modelImage.indexOf(",") + 1),
    });
    attachments.value.push({
      path,
      name,
      mime: "image/jpeg",
      preview,
    });
  } catch (cause) {
    const message = t("composer.screenshotFailed", { error: String(cause) });
    note.value = message;
    window.setTimeout(() => {
      if (note.value === message) note.value = null;
    }, 9000);
  } finally {
    capturing.value = false;
  }
}

/**
 * The field grows with what is typed and stops at ten lines — long enough for a real prompt,
 * short enough that the conversation stays on screen. Beyond that it scrolls.
 */
const MAX_ROWS = 10;
const LINE_HEIGHT = 19.5;
const box = ref<HTMLTextAreaElement | null>(null);

function autoGrow(): void {
  const element = box.value;
  if (!element) return;
  // Before the first layout — and while the main window is still hidden behind the splash —
  // the field has no box to measure. `scrollHeight` then reports the maximum, and the empty
  // composer would start 195 px tall instead of 42 and stay there until the first keystroke.
  if (element.getBoundingClientRect().width === 0) return;
  // Nothing typed: the stylesheet's own height is right, and it is the one place that knows
  // how tall an empty line is. An inline height left from a longer draft must go.
  if (!element.value.trim()) {
    element.style.height = "";
    return;
  }
  element.style.height = "auto";
  element.style.height = `${Math.min(element.scrollHeight, MAX_ROWS * LINE_HEIGHT)}px`;
}

function reset(): void {
  text.value = "";
  attachments.value = [];
  // Sent means the draft is done; keeping it would restore a message that already went out.
  cancelDraftSave();
  forgetDraft(activeSessionId.value);
  void nextTick(() => {
    if (box.value) box.value.style.height = "auto";
  });
}
const model = ref(settings.model);

const canSend = computed(() => text.value.trim().length > 0 && !running.value);

watch(model, (value) => {
  settings.model = value;
});

// A mode change or the first-start dialog can move the model; the select holds a local ref,
// so it has to follow or it keeps showing the label it was mounted with.
watch(
  () => settings.model,
  (value) => {
    if (value !== model.value) model.value = value;
  },
);

function submit() {
  const value = text.value.trim();
  if (!value || running.value) return;
  const outgoing = attachments.value.map((item) => ({ ...item }));
  reset();
  void sendPrompt(value, outgoing);
}

/**
 * The permission select is the three-stage rudder: read-only, ask before each action, or
 * full access. Choosing full goes through the consent dialog; ask/read apply immediately.
 * The watcher reverts the select to the previous stage while the dialog is open, and the
 * flag keeps the confirmed full grant from opening the dialog a second time.
 */
let confirmingFull = false;

watch(
  () => settings.permission,
  (next, previous) => {
    if (next !== "full" || confirmingFull) {
      confirmingFull = false;
      return;
    }
    settings.permission = previous ?? "ask";
    rudderOpen.value = true;
  },
);

function confirmRudder(): void {
  confirmingFull = true;
  settings.permission = "full";
  rudderOpen.value = false;
}

function cancelRudder(): void {
  rudderOpen.value = false;
}
</script>

<template>
  <div class="composer-wrap">
    <form class="composer" @submit.prevent="submit">
      <!-- Attachments sit above the field, where they were added. -->
      <div v-if="attachments.length" class="attachments">
        <div v-for="(item, index) in attachments" :key="item.path" class="attachment">
          <img :src="item.preview" :alt="item.name" :title="item.name" />
          <button
            class="remove"
            type="button"
            :title="t('composer.removeAttachment')"
            @click="removeAttachment(index)"
          >
            <Icon name="x" :size="11" />
          </button>
        </div>
      </div>

      <textarea
        ref="box"
        v-model="text"
        class="input"
        rows="1"
        :placeholder="t('composer.placeholder')"
        @keydown.enter.exact.prevent="submit"
        @input="autoGrow"
        @paste="onPaste"
      ></textarea>

      <p v-if="note" class="note">{{ note }}</p>

      <div class="row">
        <div class="left">
          <button class="chip" type="button" :title="t('composer.attach')" @click="fileInput?.click()">
            <Icon name="plus" :size="14" />
          </button>
          <button
            class="chip"
            type="button"
            :title="t('composer.screenshot')"
            :disabled="capturing"
            @click="captureScreen"
          >
            <Icon name="eye" :size="14" />
          </button>
          <input
            ref="fileInput"
            class="file-input"
            type="file"
            accept="image/*"
            multiple
            @change="onFiles"
          />
          <label
            class="chip permission"
            :class="{ on: settings.permission === 'full' }"
            :title="t('permission.hint')"
          >
            <Icon name="rudder" :size="14" />
            <select v-model="settings.permission">
              <option value="read">{{ t("permission.read") }}</option>
              <option value="ask">{{ t("permission.ask") }}</option>
              <option value="full">{{ t("permission.full", { name: partnerDisplayName }) }}</option>
            </select>
          </label>
        </div>

        <div class="right">
          <ContextRing @compacted="onCompacted" />
          <select v-model="model" class="model" :title="`${t('composer.model')}: ${modelLabel(model)} (${model})`">
            <option
              v-if="!modelChoices().some((entry) => entry.id === model)"
              :value="model"
            >
              {{ model }}
            </option>
            <option v-for="entry in modelChoices()" :key="entry.id" :value="entry.id">
              {{ entry.name }}
            </option>
          </select>

          <button
            v-if="running"
            class="send abort"
            type="button"
            :title="t('engine.abort')"
            @click="abortTurn"
          >
            <Icon name="x" :size="15" />
          </button>
          <button
            v-else
            class="send"
            type="submit"
            :disabled="!canSend"
            :title="t('composer.send')"
          >
            <Icon name="send" :size="16" />
          </button>
        </div>
      </div>
    </form>

    <p v-if="treeError" class="workspace-warning">
      <span>{{ treeError }}</span>
      <button type="button" @click="openSettings('general')">
        {{ t("workspace.openSettings") }}
      </button>
    </p>

    <p class="disclaimer">
      {{ engineReady ? t("composer.disclaimer") : t("engine.needsKey") }}
    </p>

    <RudderDialog
      :open="rudderOpen"
      :name="partnerDisplayName"
      @confirm="confirmRudder"
      @cancel="cancelRudder"
    />
  </div>
</template>

<style scoped>
/* Its own surface, so the message that scrolls up behind it is not "cut through" it, and
   the same 70% column as the list above. */
.composer-wrap {
  /* The message column reserves 6px for its scrollbar, so this box mirrors that: both have
     the same visible left and right edge instead of being "the same width" in numbers only.
     The width comes from the shared reading column in `AgentPanel.vue`. */
  width: calc(var(--chat-width, 70%) - 6px);
  max-width: 100%;
  margin-inline: auto;
  padding: 8px 6px 10px;
  border-top: 1px solid var(--border);
  /* This layer is what hides the message list behind the composer. Without it the watermark
     and the last messages show through, which looks like the composer is transparent. */
  background: var(--bg);
}

/* The controls row is a container, so it can react to its own width rather than to the
   viewport: with both side panels open the composer is much narrower than the window. */
.composer {
  container-type: inline-size;
  container-name: composer;
  border: 1px solid var(--border-strong);
  border-radius: 12px;
  background: var(--bg-composer);
  padding: 10px 12px 8px;
}

.note {
  margin: 0 0 6px;
  color: var(--text-muted);
  font-size: 11.5px;
}

.attachments {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 8px;
}

.attachment {
  position: relative;
  width: 54px;
  height: 54px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  overflow: hidden;
}

.attachment img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.remove {
  position: absolute;
  top: 1px;
  right: 1px;
  display: grid;
  place-items: center;
  width: 16px;
  height: 16px;
  border: none;
  border-radius: 50%;
  background: rgb(0 0 0 / 65%);
  color: #fff;
  cursor: pointer;
}

.file-input {
  display: none;
}

.input {
  width: 100%;
  min-height: 42px;
  max-height: 195px;
  overflow-y: auto;
  resize: none;
  border: none;
  outline: none;
  background: transparent;
  color: var(--text);
  font-size: 13px;
  line-height: 1.5;
}

.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-top: 6px;
}

.left,
.right {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.chip {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 26px;
  padding: 0 9px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-muted);
  font-size: 12px;
}

/* Only the permission control may shrink: its label is the part that can give way without
   losing a control. The select itself never wraps into a vertical stack. */
.permission {
  min-width: 0;
}

.permission select {
  min-width: 0;
  /* Room for "Vor Änderungen fragen"; the old 150px cut the longer stage name. */
  max-width: 190px;
  border: none;
  background: transparent;
  color: inherit;
  font: inherit;
  outline: none;
  cursor: pointer;
}

.permission select option {
  background: var(--bg-elevated);
  color: var(--text);
}

.permission.on {
  color: var(--accent);
}

.chip:hover {
  background: var(--bg-hover);
  color: var(--text-strong);
}

.chip:disabled {
  cursor: default;
  opacity: 0.45;
}

.chip.on {
  color: var(--accent);
}

.model {
  flex: 0 1 150px;
  min-width: 92px;
  height: 26px;
  padding: 0 6px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text);
  font-size: 12px;
  text-align: right;
}

.model:focus {
  outline: none;
  background: var(--bg-input);
}

/* Narrow composer (both side panels open): the permission label goes, the select keeps a
   readable model name. The controls stay on one row instead of spilling out of the bubble. */
@container composer (max-width: 440px) {
  /* Keep the icon and the control; only the long stage label gives way. */
  .permission select {
    max-width: 110px;
  }

  .model {
    flex-basis: 118px;
  }
}

.send {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  border: none;
  border-radius: 50%;
  background: var(--bg-active);
  color: var(--text-strong);
}

.send:disabled {
  opacity: 0.4;
  cursor: default;
}

.send:not(:disabled):hover {
  background: var(--accent-strong);
  color: var(--accent-contrast);
}

.send.abort {
  background: var(--danger);
  color: var(--accent-contrast);
}

.disclaimer {
  margin: 8px 0 0;
  color: var(--text-muted);
  font-size: 11px;
  text-align: center;
}

/* The workspace problem belongs where the user looks while typing, not only in the tree. */
.workspace-warning {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin: 8px 0 0;
  color: var(--danger);
  font-size: 11px;
  text-align: center;
}

.workspace-warning button {
  padding: 2px 9px;
  border: 1px solid var(--danger);
  border-radius: 999px;
  background: transparent;
  color: var(--danger);
  font-size: 11px;
}

.workspace-warning button:hover {
  background: var(--bg-hover);
}
</style>
