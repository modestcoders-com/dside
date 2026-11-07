<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import Resizer from "./components/Resizer.vue";
import TitleBar from "./components/TitleBar.vue";
import LeftRail from "./components/LeftRail.vue";
import SearchDialog from "./components/SearchDialog.vue";
import WebDesignPanel from "./components/WebDesignPanel.vue";
import AgentPanel from "./components/agent/AgentPanel.vue";
import WorkArea from "./components/workarea/WorkArea.vue";
import SettingsPage from "./components/settings/SettingsPage.vue";
import FirstRunDialog from "./components/FirstRunDialog.vue";
import DeepSeekWhale from "./components/DeepSeekWhale.vue";
import { deepSeekMark } from "./components/deepseek-mark";
import { trackWindowTitle } from "./composables/useWindowTitle";
import { firstRun } from "./stores/settings";
import { partnerDisplayName, partnerReveal, refreshPartnerName } from "./stores/partner";
import Icon from "./components/Icon.vue";
import { locale, t } from "./i18n";
import {
  CENTER_MIN,
  railAuto,
  railCollapsed,
  railWidth,
  searchOpen,
  setRailWidth,
  setWorkWidth,
  view,
  workAuto,
  workWidth,
} from "./stores/ui";
import { visible as workAreaVisible } from "./stores/workArea";
import { useResponsiveLayout } from "./composables/useResponsiveLayout";
import { useShortcuts } from "./composables/useShortcuts";

/**
 * The panels the grid actually draws. The manual flags are what the user chose; the `Auto`
 * flags mean "there is no room right now". Keeping them apart is what lets a narrow window
 * drop the rail first without overwriting the preference for the next wide one.
 */
const railVisible = computed(() => !railCollapsed.value && !railAuto.value);
const workVisible = computed(() => workAreaVisible.value && !workAuto.value);

useResponsiveLayout();

const body = ref<HTMLElement | null>(null);

/**
 * The splash is a separate, chrome-less Tauri window that loads this same page. It only
 * renders the brand; the main window stays hidden until Rust closes the splash.
 */
let windowLabel = "main";
try {
  windowLabel = getCurrentWindow().label;
} catch {
  // running in a plain browser — treat it as the main window
}
const isSplash = ref(windowLabel === "splash");
/** Set once the window is on screen; the splash animations hang off this class. */
const splashShown = ref(false);

useShortcuts(() => !isSplash.value);

/**
 * Launch splash. The bar is theatre — nothing is downloaded — but a bar that moves
 * unevenly reads as "working" while a smooth one reads as "fake". The curve lives in CSS
 * (`splash-progress`), because `requestAnimationFrame` does not fire reliably in a splash
 * window under Wayland: the first version stalled here and never handed over.
 */
const SPLASH_MS = 7000;

/** The splash stays hidden until its artwork is ready, then holds for the full run. */
async function finishSplash() {
  const say = (text: string) => {
    void invoke("log_message", { text }).catch(() => undefined);
  };

  // The mark is inline SVG now, so there is no image to wait for — only the first render.
  say("splash: waiting for artwork");
  await nextTick();
  say("splash: artwork ready");

  await invoke("splash_visible").catch(() => undefined);
  splashShown.value = true;
  say("splash: visible");

  await new Promise<void>((resolve) => {
    window.setTimeout(resolve, SPLASH_MS);
  });
  say("splash: progress done");

  await invoke("splash_ready").catch(() => undefined);
  say("splash: handed over");
}

/**
 * The tray menu belongs to the six catalogs, so the words are pushed from here: once at start,
 * and again whenever the user switches language. The backend only holds the label strings.
 */
function pushTrayLabels(): void {
  void invoke("tray_set_labels", {
    open: t("tray.open"),
    quit: t("tray.quit"),
  }).catch(() => undefined);
}

watch(locale, pushTrayLabels);

onMounted(() => {
  trackWindowTitle();
  if (isSplash.value) {
    void finishSplash();
    return;
  }
  pushTrayLabels();
  void invoke("log_message", { text: "main window: mounted" }).catch(() => undefined);
  // Crossing the trust threshold is a moment, and it happens mid-conversation: the engine
  // announces it once, the banner says it, the partner page shows the rest.
  void listen<{ name?: string }>("trust://revealed", (event) => {
    partnerReveal.value = event.payload?.name?.trim() || "";
    void refreshPartnerName();
  }).catch(() => undefined);
});

/**
 * The grid always keeps three columns, with a hidden side collapsed to 0 %. That is what makes
 * the change animatable: adding or removing a column would snap, changing its width slides.
 * The panels stay mounted while hidden (inert, so nothing inside can be focused), which also
 * keeps their state — the tree, the terminal, the open tab.
 */
const bodyStyle = computed(() => ({
  gridTemplateColumns: `${railVisible.value ? railWidth.value : 0}% minmax(0, 1fr) ${
    workVisible.value ? workWidth.value : 0
  }%`,
}));

/**
 * A collapsed column still leaves its 10 px gap behind. Pulling the middle panel over by that
 * much keeps it flush with the window edge while the rail slides away — animated together with
 * the column, so both edges move as one.
 */
const centerStyle = computed(() => ({
  marginLeft: railVisible.value ? "0px" : "-10px",
  marginRight: workVisible.value ? "0px" : "-10px",
}));

function startDrag(which: "rail" | "work", event: PointerEvent) {
  const container = body.value;
  if (!container) return;
  const total = container.clientWidth;
  if (total <= 0) return;

  // Dragging must be immediate; the slide transition would lag behind the pointer.
  document.body.classList.add("is-resizing");
  const startX = event.clientX;
  const startRail = railWidth.value;
  const startWork = workWidth.value;
  const handle = event.currentTarget as HTMLElement;
  handle.setPointerCapture(event.pointerId);

  const onMove = (move: PointerEvent) => {
    const delta = ((move.clientX - startX) / total) * 100;

    if (which === "rail") {
      const available = 100 - CENTER_MIN - (workVisible.value ? workWidth.value : 0);
      setRailWidth(Math.min(startRail + delta, available));
      return;
    }

    const requested = startWork - delta;
    const railSpace = railVisible.value ? railWidth.value : 0;

    // Free up room by collapsing the left rail when the center would be squeezed.
    if (requested + railSpace + CENTER_MIN > 100 && railVisible.value) {
      railCollapsed.value = true;
    }

    const available = 100 - CENTER_MIN - (railVisible.value ? railWidth.value : 0);
    setWorkWidth(Math.min(requested, available));
  };

  const onUp = () => {
    document.body.classList.remove("is-resizing");
    handle.removeEventListener("pointermove", onMove);
    handle.removeEventListener("pointerup", onUp);
    handle.removeEventListener("pointercancel", onUp);
  };

  handle.addEventListener("pointermove", onMove);
  handle.addEventListener("pointerup", onUp);
  handle.addEventListener("pointercancel", onUp);
}
</script>

<template>
  <div v-if="isSplash" class="splash-window">
    <div class="splash-disc" :class="{ animate: splashShown }">
      <DeepSeekWhale class="splash-whale" :height="110" />
      <span class="splash-mark" v-html="deepSeekMark"></span>
      <div class="splash-bar"><span></span></div>
    </div>
  </div>

  <template v-else>
  <SettingsPage v-if="view === 'settings'" @close="view = 'chat'" />

  <div v-else class="ide">
    <TitleBar />
    <div class="stage">
      <div ref="body" class="body" :style="bodyStyle">
        <LeftRail
          class="card side-panel left-panel"
          :class="{ hidden: !railVisible }"
          :aria-hidden="railVisible ? undefined : 'true'"
          :inert="railVisible ? undefined : true"
        />
        <AgentPanel class="card center-panel" :style="centerStyle" />
        <WorkArea
          class="card side-panel right-panel"
          :class="{ hidden: !workVisible }"
          :aria-hidden="workVisible ? undefined : 'true'"
          :inert="workVisible ? undefined : true"
        />

        <Resizer
          v-if="railVisible"
          align="left"
          :style="{ left: `calc(${railWidth}% + 5px)` }"
          @start="(event) => startDrag('rail', event)"
        />
        <Resizer
          v-if="workVisible"
          align="right"
          :style="{ right: `calc(${workWidth}% + 5px)` }"
          @start="(event) => startDrag('work', event)"
        />
      </div>
    </div>
  </div>

    <div v-if="partnerReveal !== null" class="reveal-banner" role="status">
      <Icon name="sparkle" :size="14" />
      <span>{{ t("partner.revealBanner", { name: partnerReveal || partnerDisplayName }) }}</span>
      <button type="button" :aria-label="t('common.close')" @click="partnerReveal = null">
        <Icon name="x" :size="12" />
      </button>
    </div>

    <FirstRunDialog v-if="firstRun" />
    <SearchDialog v-if="searchOpen" />
    <WebDesignPanel />
  </template>
</template>

<style scoped>
/* A quiet banner, not a dialog: the reveal should not interrupt the work it came from. */
.reveal-banner {
  position: fixed;
  left: 50%;
  bottom: 24px;
  z-index: 90;
  display: flex;
  align-items: center;
  gap: 8px;
  max-width: min(520px, calc(100vw - 32px));
  padding: 9px 10px 9px 12px;
  border: 1px solid var(--accent);
  border-radius: var(--radius);
  background: var(--bg-panel);
  color: var(--text);
  font-size: 12px;
  transform: translateX(-50%);
  box-shadow: 0 12px 40px rgb(0 0 0 / 45%);
}

.reveal-banner span {
  flex: 1;
  min-width: 0;
}

.reveal-banner button {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-muted);
}

.reveal-banner button:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.ide {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  background: var(--bg-app);
}

.stage {
  flex: 1;
  min-height: 0;
  display: flex;
  padding: 0 10px 10px;
}

.body {
  position: relative;
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-rows: minmax(0, 1fr);
  gap: 10px;
  transition: grid-template-columns 320ms cubic-bezier(0.22, 1, 0.36, 1);
}

/* A side panel slides out of its edge and fades while its column collapses. 320 ms reads as a
   deliberate movement without making the next click wait; the fork this came from used 1.15 s,
   which is lovely to watch and slow to live with. */
.side-panel {
  min-width: 0;
  overflow: hidden;
  transition:
    opacity 220ms ease,
    transform 320ms cubic-bezier(0.22, 1, 0.36, 1),
    border-width 320ms cubic-bezier(0.22, 1, 0.36, 1);
}

.left-panel {
  transform-origin: left center;
}

.right-panel {
  transform-origin: right center;
}

.left-panel.hidden {
  transform: translateX(-18px);
}

.right-panel.hidden {
  transform: translateX(18px);
}

.side-panel.hidden {
  border-width: 0;
  opacity: 0;
  pointer-events: none;
}

.center-panel {
  transition: margin 320ms cubic-bezier(0.22, 1, 0.36, 1);
}

/* While a divider is dragged, every transition on the layout is in the way. */
body.is-resizing .body,
body.is-resizing .side-panel,
body.is-resizing .center-panel {
  transition: none;
}

@media (prefers-reduced-motion: reduce) {
  .body,
  .side-panel,
  .center-panel {
    transition-duration: 1ms;
  }
}

/* The splash window is chrome-less: it shows the brand and nothing else. */
/**
 * The splash draws a disc on a transparent window, so what appears is a circle rather than a
 * rectangle sitting in the middle of the screen. If the compositor refuses transparency the
 * disc still reads as a disc — a dark square behind it is the worst case.
 */
.splash-window {
  display: grid;
  place-items: center;
  height: 100vh;
  background: transparent;
}

.splash-disc {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 14px;
  width: 300px;
  height: 300px;
  border: 1px solid var(--border-strong);
  border-radius: 50%;
  background: var(--bg-app);
  box-shadow: 0 18px 60px rgb(0 0 0 / 45%);
  overflow: hidden;
}

.splash-disc .splash-whale {
  position: relative;
  z-index: 0;
  top: 40px;
  right: 10px;
  transform-origin: 50% 55%;
  will-change: transform;
}

/* The whale grows towards the edge of the disc while the progress bar runs — the same 7
   seconds, so the two read as one movement. The disc clips it. Windows with animation
   effects off reports `prefers-reduced-motion: reduce`; the splash is the app's one
   deliberate movement, so it runs regardless. Both animations start only once the window
   is shown (`animate`), not while the webview is still hidden. */
.splash-disc.animate .splash-whale {
  animation: whale-zoom 7s linear forwards;
}

@keyframes whale-zoom {
  from {
    transform: scale(1);
  }

  to {
    transform: scale(1.85);
  }
}

/* Text and bar stay above the whale, whatever size it reaches. */
.splash-mark,
.splash-bar {
  position: relative;
  z-index: 1;
}

.splash-mark {
  display: inline-flex;
  color: var(--text-strong);
}

.splash-mark :deep(svg) {
  display: block;
  height: 22px;
  width: auto;
}

.splash-bar {
  width: 170px;
  height: 3px;
  border-radius: 999px;
  background: var(--bg-elevated);
  overflow: hidden;
}

.splash-bar span {
  display: block;
  height: 100%;
  background: var(--accent-whale);
  border-radius: 999px;
}

.splash-disc.animate .splash-bar span {
  animation: splash-progress 7s linear forwards;
}

/* Uneven on purpose: plateaus read as work, jumps as progress. */
@keyframes splash-progress {
  0% {
    width: 0%;
  }
  6% {
    width: 7%;
  }
  11% {
    width: 8%;
  }
  18% {
    width: 24%;
  }
  22% {
    width: 25%;
  }
  31% {
    width: 46%;
  }
  38% {
    width: 50%;
  }
  49% {
    width: 69%;
  }
  57% {
    width: 71%;
  }
  67% {
    width: 86%;
  }
  75% {
    width: 87%;
  }
  85% {
    width: 96%;
  }
  93% {
    width: 99%;
  }
  100% {
    width: 100%;
  }
}
</style>
