<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { t } from "../../i18n";
import { modelChoices } from "../../stores/models";
import { settings } from "../../stores/settings";
import { invoke } from "@tauri-apps/api/core";
import { clearLastUsage, lastUsage } from "../../composables/useEngine";
import { activeSessionId } from "../../stores/sessions";

const emit = defineEmits<{ compacted: [folded: number | null, error: string | null] }>();

const working = ref(false);

/**
 * What the next request would carry, asked from the engine — the same figure the automatic
 * compression uses. It prefers the provider's own report while that report still describes
 * this history, and falls back to an estimate after a compression, when it no longer does.
 *
 * There used to be a second source here: the raw `promptTokens` of the last request. It is
 * exact, but it survives restarts and it describes a history that compression replaces — so
 * the ring kept showing the old size and a successful compression looked like a no-op.
 */
const status = ref<{ used: number; basis: string } | null>(null);

async function refresh(): Promise<void> {
  try {
    status.value = await invoke<{ used: number; basis: string }>("context_status", {
      sessionId: activeSessionId.value,
    });
  } catch {
    // The ring keeps whatever it had; a missing number is better than a wrong one.
  }
}

onMounted(() => void refresh());
// Every turn ends with a usage report — that is the moment the measurement moved on.
watch(lastUsage, () => void refresh());

/**
 * Compressing is an explicit act: it costs a model call, it changes what the model sees, and
 * the user should be the one to decide. Automatic compression happens on its own (when a turn
 * would be too large); this is the button for "do it now".
 */
async function compact(): Promise<void> {
  if (working.value) return;
  working.value = true;
  try {
    const report = await invoke<{ folded: number } | null>("session_compact", {
      sessionId: activeSessionId.value,
    });
    // The stored measurement now describes a history that is gone; drop it before asking for
    // the new size, or a later start would restore the old number again.
    clearLastUsage();
    await refresh();
    emit("compacted", report?.folded ?? null, null);
  } catch (error) {
    // A failure is not "nothing to compress": saying so would hide an error behind a phrase
    // that sounds like an answer.
    emit("compacted", null, String(error));
  } finally {
    working.value = false;
  }
}

/**
 * How much of the model's context is used, as a ring. The number comes from the provider's
 * own usage report for the last request, the limit from the model list — nothing here is
 * invented. Clicking it compresses the older part of the conversation now; the ring itself
 * only displays.
 */
const RADIUS = 7;
const CIRCUMFERENCE = 2 * Math.PI * RADIUS;

const limit = computed(
  () => modelChoices().find((entry) => entry.id === settings.model)?.contextWindow ?? 0,
);

/** The figure the engine gave, and whether it is a measurement or an estimate of what is left. */
const used = computed<number | null>(() => status.value?.used ?? null);

const isEstimated = computed(() => status.value !== null && status.value.basis !== "measured");

const share = computed(() => {
  if (!limit.value || used.value === null) return 0;
  return Math.min(1, used.value / limit.value);
});

const percent = computed(() => Math.round(share.value * 100));

const hint = computed(() => {
  if (used.value === null) return t("composer.contextUnknown");
  if (!limit.value) return t("composer.tokens", { used: used.value });
  if (isEstimated.value) {
    return t("composer.contextHintEstimated", {
      used: used.value,
      total: limit.value,
      percent: percent.value,
    });
  }
  return t("composer.contextHint", {
    used: used.value,
    total: limit.value,
    percent: percent.value,
  });
});

const colour = computed(() => {
  if (percent.value >= 85) return "var(--danger)";
  if (percent.value >= 60) return "var(--accent)";
  return "var(--text-muted)";
});
</script>

<template>
  <button
    class="ring"
    type="button"
    :title="`${hint} — ${t('composer.compact')}`"
    :disabled="working"
    @click="compact"
  >
    <svg viewBox="0 0 18 18" width="18" height="18" aria-hidden="true">
      <circle cx="9" cy="9" :r="RADIUS" fill="none" stroke="var(--border-strong)" stroke-width="2" />
      <circle
        cx="9"
        cy="9"
        :r="RADIUS"
        fill="none"
        :stroke="colour"
        stroke-width="2"
        stroke-linecap="round"
        :stroke-dasharray="CIRCUMFERENCE"
        :stroke-dashoffset="CIRCUMFERENCE * (1 - share)"
        transform="rotate(-90 9 9)"
      />
    </svg>
  </button>
</template>

<style scoped>
.ring {
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  flex: none;
  padding: 0;
  border: none;
  background: transparent;
  cursor: pointer;
}

.ring:disabled {
  cursor: progress;
}
</style>
