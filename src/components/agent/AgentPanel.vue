<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import Icon from "../Icon.vue";
import MessageList from "./MessageList.vue";
import Composer from "./Composer.vue";
import { t } from "../../i18n";
import { activeSessionId, messages } from "../../stores/sessions";
import { settings } from "../../stores/settings";
import {
  answerAction,
  historyHasMore,
  initEngine,
  loadOlderHistory,
  pendingAction,
  running,
} from "../../composables/useEngine";

const composer = ref<InstanceType<typeof Composer> | null>(null);

/**
 * After a turn the next sentence usually goes into the same field. Put the cursor back —
 * but never take it away from a field the user is already typing in (for example while
 * the turn is still streaming and they are writing the next message).
 */
watch(running, (isRunning) => {
  if (isRunning) return;
  const active = document.activeElement as HTMLElement | null;
  if (
    active &&
    (active.tagName === "INPUT" ||
      active.tagName === "TEXTAREA" ||
      active.tagName === "SELECT" ||
      active.isContentEditable)
  ) {
    return;
  }
  void nextTick(() => composer.value?.focus());
});

const freeAnswer = ref("");
/**
 * The engine gives a question five minutes, and the UI said nothing about it — someone who
 * steps away for a moment comes back to a card that is already dead. The countdown makes the
 * deadline visible before it matters.
 */
/** The wait the engine was told to use, so the countdown cannot promise a different deadline. */
const askSeconds = computed(() =>
  Math.max(30, Math.min(3600, Math.round(settings.askTimeoutMinutes * 60))),
);
const secondsLeft = ref(askSeconds.value);
let askTimer: number | undefined;

function startCountdown(): void {
  window.clearInterval(askTimer);
  secondsLeft.value = askSeconds.value;
  askTimer = window.setInterval(() => {
    secondsLeft.value = Math.max(0, secondsLeft.value - 1);
    if (secondsLeft.value === 0) window.clearInterval(askTimer);
  }, 1000);
}

watch(pendingAction, (action) => {
  answerError.value = null;
  if (action) startCountdown();
  else window.clearInterval(askTimer);
});

const countdownLabel = computed(() => {
  const left = secondsLeft.value;
  return `${Math.floor(left / 60)}:${String(left % 60).padStart(2, "0")}`;
});
/** What went wrong with the last answer attempt, shown in the card until the next try. */
const answerError = ref<string | null>(null);

/**
 * Sends an answer and keeps the text until it is known to have arrived. The engine gives a
 * question five minutes; after that the text used to vanish silently — typed, sent, gone.
 */
async function answer(option: string): Promise<boolean> {
  const action = pendingAction.value;
  if (!action) return false;
  answerError.value = null;
  try {
    await answerAction(action.id, option);
    return true;
  } catch (cause) {
    answerError.value = String(cause);
    return false;
  }
}

async function submitFree() {
  const text = freeAnswer.value.trim();
  if (!text) return;
  const delivered = await answer(text);
  // Cleared only on success: a failed send keeps the sentence in the field, where it can be
  // sent again — or moved to the composer — instead of being lost.
  if (delivered) freeAnswer.value = "";
}

/** Takes the typed answer out of the expired card and into the composer. */
function keepAnswerAsMessage(): void {
  const text = freeAnswer.value.trim();
  if (!text) return;
  composer.value?.setDraft(text);
  freeAnswer.value = "";
  answerError.value = null;
}

const scrollEl = ref<HTMLElement | null>(null);
const listEl = ref<InstanceType<typeof MessageList> | null>(null);
const following = ref(true);
let observer: ResizeObserver | null = null;

/**
 * What can make the list taller. Reasoning is part of it: while the companion thinks, that
 * text grows too, and a block the reader opened during the turn visibly pushes everything
 * down — leaving it out meant the list did not follow during exactly that step.
 */
const messagesChanged = computed(() => {
  const list = messages.value;
  const last = list[list.length - 1];
  const tools = last?.tools ?? [];
  const toolBytes = tools.reduce((sum, tool) => sum + (tool.output?.length ?? 0), 0);
  const parts = last?.parts?.length ?? 0;
  return `${list.length}|${last?.text.length ?? 0}|${last?.reasoning?.length ?? 0}|${parts}|${tools.length}|${toolBytes}`;
});

function isAtBottom(element: HTMLElement): boolean {
  return element.scrollHeight - element.scrollTop - element.clientHeight < 48;
}

/**
 * The line the reader is on, kept across width changes. Dragging a divider or the window edge
 * re-wraps every paragraph, so the same `scrollTop` shows different text: measured here, the
 * content under the cursor slid by a message while `scrollTop` did not move at all. The webview
 * does not compensate, so the panel remembers the top row and its distance from the edge.
 */
let anchorId: string | null = null;
let anchorOffset = 0;

/**
 * The first row whose bottom is still below the top edge. Binary search: a long conversation
 * has thousands of rows, and this runs on every scroll instead of only on a resize.
 */
function topRow(element: HTMLElement): HTMLElement | null {
  const rows = element.querySelectorAll<HTMLElement>("[data-message-id]");
  if (rows.length === 0) return null;
  const top = element.getBoundingClientRect().top;
  let low = 0;
  let high = rows.length - 1;
  let found: HTMLElement | null = null;
  while (low <= high) {
    const mid = (low + high) >> 1;
    const row = rows[mid];
    if (row.getBoundingClientRect().bottom > top) {
      found = row;
      high = mid - 1;
    } else {
      low = mid + 1;
    }
  }
  return found;
}

function rememberAnchor(): void {
  const element = scrollEl.value;
  if (!element) return;
  const row = topRow(element);
  if (!row) return;
  anchorId = row.dataset.messageId ?? null;
  anchorOffset = row.getBoundingClientRect().top - element.getBoundingClientRect().top;
}

/** Puts the remembered row back where it was. Runs after a resize, never during a scroll. */
function restoreAnchor(): void {
  const element = scrollEl.value;
  if (!element || !anchorId) return;
  const row = element.querySelector<HTMLElement>(`[data-message-id="${anchorId}"]`);
  if (!row) return;
  const offset = row.getBoundingClientRect().top - element.getBoundingClientRect().top;
  const delta = offset - anchorOffset;
  if (Math.abs(delta) < 1) return;
  // Moving the content up by `delta` is the same as scrolling down by it.
  scrollBy(delta);
}

/**
 * What we write to `scrollTop` ourselves. Scroll events caused that way are not the reader
 * leaving the bottom — mistaking them for that turned following off mid-resize, and the last
 * message then slipped below the edge for good.
 */
let selfTop: number | null = null;

function scrollBy(delta: number): void {
  const element = scrollEl.value;
  if (!element) return;
  element.scrollTop += delta;
  selfTop = element.scrollTop;
}

/** Glues the newest line to the bottom edge — the "sticky last post". */
function pinToBottom(): void {
  const element = scrollEl.value;
  if (!element) return;
  element.scrollTop = element.scrollHeight;
  selfTop = element.scrollTop;
}

/**
 * Scrolls to the newest line. Deliberately not `requestAnimationFrame`: in this webview (splash
 * included) rAF can go quiet, which is why the list stopped following until it was nudged.
 * `nextTick` runs after Vue has written the DOM, which is exactly the moment we need.
 */
function queueFollow(): void {
  void nextTick(pinToBottom);
}

/** The scroll position at the previous event, to tell a reader's scroll from growth. */
let lastTop: number | null = null;

function onScroll(): void {
  const element = scrollEl.value;
  if (!element) return;
  const top = element.scrollTop;
  const moved = lastTop !== null && Math.abs(top - lastTop) > 1;
  lastTop = top;

  const ours = selfTop !== null && Math.abs(top - selfTop) <= 1;
  if (ours) {
    // The layout may have grown after our write; while following, take the bottom again.
    if (following.value && !isAtBottom(element)) pinToBottom();
    return;
  }

  // A scroll event whose position did not change is not a scroll: the content grew and the
  // webview reported it anyway. Reading that as "the reader left the bottom" is what turned
  // following off in the middle of an answer — and then the next line stayed below the edge.
  if (!moved) {
    if (following.value) pinToBottom();
    return;
  }

  following.value = isAtBottom(element);
  selfTop = null;
  // A correction is measured against the remembered line; recording the result of an
  // integer-rounded scroll would let a pixel slip in on every resize and add up.
  rememberAnchor();
}

const loadingOlder = ref(false);

/**
 * Older messages are prepended, so the scroll height grows above the current line. Keep that
 * line where it was — otherwise reading back through a long conversation would jump to a new
 * place with every page.
 */
async function loadOlder(): Promise<void> {
  const element = scrollEl.value;
  if (!element || loadingOlder.value) return;
  const beforeHeight = element.scrollHeight;
  const beforeTop = element.scrollTop;
  // Appending is what the list follows; prepending must not trigger a jump to the bottom.
  following.value = false;
  loadingOlder.value = true;
  try {
    await loadOlderHistory(activeSessionId.value);
    await nextTick();
    element.scrollTop = beforeTop + (element.scrollHeight - beforeHeight);
  } finally {
    loadingOlder.value = false;
  }
}

function jumpToBottom(smooth: boolean): void {
  const element = scrollEl.value;
  if (!element) return;
  following.value = true;
  if (smooth) {
    element.scrollTo({ top: element.scrollHeight, behavior: "smooth" });
    return;
  }
  pinToBottom();
}

watch(
  messagesChanged,
  () => {
    const list = messages.value;
    // The user just acted: always follow their own message.
    if (list[list.length - 1]?.role === "user") following.value = true;
    if (following.value) queueFollow();
  },
  { flush: "post" },
);

watch(activeSessionId, () => {
  following.value = true;
  queueFollow();
});

/**
 * A window resize changes two things at once: the column re-wraps (content grows) and the
 * composer may take more rows (the scroll area shrinks). The list alone does not see the
 * second, so both are observed — measured here, at 1000 px the last line sat 59 px below the
 * edge and the jump button appeared.
 */
onMounted(() => {
  // Markdown, images and fonts change the height after the message was appended. As long as
  // the reader is at the bottom, follow that growth too. When they are not, hold their line:
  // a wider or narrower column re-wraps everything above it.
  const list = (listEl.value?.$el as HTMLElement | undefined) ?? scrollEl.value;
  if (typeof ResizeObserver !== "undefined") {
    observer = new ResizeObserver(() => {
      if (following.value) {
        // The layout is already current here, so this needs no deferral.
        pinToBottom();
        return;
      }
      restoreAnchor();
    });
    if (list) observer.observe(list);
    if (scrollEl.value) observer.observe(scrollEl.value);
  }
  window.addEventListener("resize", onWindowResize);
  void initEngine().then(() => jumpToBottom(false));
});

/** The viewport can change without the list or the scroll area changing size. */
function onWindowResize(): void {
  if (following.value) pinToBottom();
}

onBeforeUnmount(() => {
  observer?.disconnect();
  observer = null;
  window.clearInterval(askTimer);
  window.removeEventListener("resize", onWindowResize);
});
</script>

<template>
  <section class="agent">
    <div class="scroll-wrap">
      <div ref="scrollEl" class="scroll" @scroll="onScroll">
        <MessageList
          ref="listEl"
          :messages="messages"
          :has-older="historyHasMore[activeSessionId] ?? false"
          :loading-older="loadingOlder"
          @load-older="loadOlder"
        />
      </div>

      <button
        v-if="!following"
        class="jump"
        type="button"
        @click="jumpToBottom(true)"
      >
        <Icon name="chevron-down" :size="13" />
        <span>{{ t("composer.scrollToBottom") }}</span>
      </button>
    </div>

    <section v-if="pendingAction" class="question">
      <div class="question-head">
        <Icon :name="pendingAction.kind === 'permission' ? 'rudder' : 'sparkle'" :size="13" />
        <span>{{
          pendingAction.kind === "permission"
            ? t("permission.requestTitle")
            : t("question.title")
        }}</span>
        <span class="question-timer" :class="{ soon: secondsLeft <= 60 }">
          {{ t("question.expiresIn", { time: countdownLabel }) }}
        </span>
      </div>
      <p
        class="question-text"
        :class="{ technical: pendingAction.kind === 'permission' }"
      >{{ pendingAction.question }}</p>

      <template v-if="pendingAction.kind === 'permission'">
        <p class="question-note">{{ t("permission.onceHint") }}</p>
        <div class="options">
          <button type="button" @click="answer('allow')">
            {{ t("permission.allow") }}
          </button>
          <button
            v-if="pendingAction.options.includes('session')"
            type="button"
            @click="answer('session')"
          >
            {{ t("permission.allowSession") }}
          </button>
          <button
            v-if="pendingAction.options.includes('always')"
            type="button"
            @click="answer('always')"
          >
            {{ t("permission.allowAlways") }}
          </button>
          <button type="button" class="deny" @click="answer('deny')">
            {{ t("permission.deny") }}
          </button>
        </div>
      </template>

      <template v-else>
        <div v-if="pendingAction.options.length" class="options">
          <button
            v-for="option in pendingAction.options"
            :key="option"
            type="button"
            @click="answer(option)"
          >
            {{ option }}
          </button>
        </div>
        <form class="free" @submit.prevent="submitFree">
          <input v-model="freeAnswer" :placeholder="t('question.placeholder')" />
          <button type="submit" :disabled="!freeAnswer.trim()">{{ t("question.send") }}</button>
        </form>
        <p v-if="answerError" class="question-error">
          {{ t("question.expired") }}
          <button
            v-if="freeAnswer.trim()"
            class="keep-answer"
            type="button"
            @click="keepAnswerAsMessage"
          >
            {{ t("question.keepText") }}
          </button>
        </p>
      </template>
    </section>

    <Composer ref="composer" />
  </section>
</template>

<style scoped>
.agent {
  display: flex;
  flex-direction: column;
  padding-inline: 16px;
  position: relative;
  /* One reading column for the list and the composer. It keeps the 70 % feel on a wide
     window but refuses to shrink below a usable width when the side panels are open —
     which is what clipped the composer controls at the minimum window size. */
  --chat-width: min(100%, max(70%, 560px), 900px);
  /* Own stacking context, so the watermark can sit *behind* the content instead of above it:
     a positioned ::before with z-index auto paints over in-flow children — which is why the
     mark showed on top of the supposedly opaque composer. */
  isolation: isolate;
}

/* Watermark behind the conversation. The DeepSeek mark, white on dark and blue on light. */
.agent::before {
  content: "";
  position: absolute;
  inset: 0;
  z-index: -1;
  background: var(--watermark) center 30% / var(--watermark-size) auto no-repeat;
  opacity: var(--watermark-opacity);
  pointer-events: none;
}

.scroll-wrap {
  position: relative;
  flex: 1;
  min-height: 0;
  display: flex;
  justify-content: center;
}

/* The scrolling column *is* the column the composer uses, so both have the same edges.
   Before, the scrollbar ate into the messages and made them a scrollbar narrower. */
.scroll {
  /* No `flex: 1` here: as a flex item its basis would win over the width, and the column
     would quietly stretch to the whole panel. */
  min-height: 0;
  width: var(--chat-width);
  margin: 0 auto;
  padding: 0 6px;
  overflow-y: auto;
  /* Our own anchoring (see the script) decides where the reader stays. The browser's
     scroll anchoring picks a different element and moves scrollTop behind our back —
     measured as a 20 px drift per resize. */
  overflow-anchor: none;
  scrollbar-width: thin;
  scrollbar-color: var(--border-strong) transparent;
  scrollbar-gutter: stable;
}

.scroll::-webkit-scrollbar {
  width: 6px;
}

.scroll::-webkit-scrollbar-thumb {
  background: var(--border-strong);
  border-radius: 3px;
}

.scroll::-webkit-scrollbar-track {
  background: transparent;
}

.jump {
  position: absolute;
  left: 50%;
  bottom: 12px;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 12px;
  border: 1px solid var(--border-strong);
  border-radius: 999px;
  background: var(--bg-elevated);
  color: var(--text-strong);
  font-size: 12px;
  box-shadow: 0 8px 20px rgba(0, 0, 0, 0.45);
}

.jump:hover {
  border-color: var(--accent);
}

.question {
  margin: 0 14px 12px;
  padding: 12px 14px;
  border: 1px solid var(--accent-strong);
  border-radius: 12px;
  background: var(--bg-elevated);
  font-size: var(--reading-font-size);
}

.question-head {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--accent);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.5px;
  text-transform: uppercase;
}

.question-text {
  margin: 8px 0 10px;
  color: var(--text-strong);
  font-size: 13px;
  line-height: 1.5;
}

.question-text.technical {
  max-height: 180px;
  overflow: auto;
  font-family: var(--mono);
  font-size: 11px;
  line-height: 1.45;
  white-space: pre-wrap;
  word-break: break-word;
}

.question-timer {
  margin-left: auto;
  color: var(--text-muted);
  font-size: 10.5px;
  font-variant-numeric: tabular-nums;
  letter-spacing: 0;
  text-transform: none;
}

.question-timer.soon {
  color: var(--danger);
}

.question-error {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin: 8px 0 0;
  color: var(--danger);
  font-size: 11px;
  line-height: 1.45;
}

.keep-answer {
  padding: 3px 9px;
  border: 1px solid var(--border-strong);
  border-radius: 999px;
  background: transparent;
  color: var(--text);
  font-size: 11px;
}

.keep-answer:hover {
  border-color: var(--accent);
  color: var(--text-strong);
}

.question-note {
  margin: 0 0 8px;
  color: var(--text-muted);
  font-size: 11px;
}

.options {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 10px;
}

.options button {
  padding: 5px 10px;
  border: 1px solid var(--border-strong);
  border-radius: 999px;
  background: transparent;
  color: var(--text);
  font-size: 12px;
}

.options button:hover {
  border-color: var(--accent);
  color: var(--text-strong);
}

.options button.deny:hover {
  border-color: var(--danger);
  color: var(--danger);
}

.free {
  display: flex;
  gap: 6px;
}

.free input {
  flex: 1;
  min-width: 0;
  height: 28px;
  padding: 0 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text);
  font-size: 12px;
}

.free button {
  height: 28px;
  padding: 0 12px;
  border: none;
  border-radius: var(--radius-sm);
  background: var(--accent-strong);
  color: var(--accent-contrast);
  font-size: 12px;
}

.free button:disabled {
  opacity: 0.45;
}
</style>
