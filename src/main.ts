import { createApp } from "vue";
import App from "./App.vue";
import "./styles/global.css";
import { initReadingSize, initSettings, initTheme } from "./stores/settings";
import { loadUiState } from "./lib/uiState";
import { initWorkArea } from "./stores/workArea";
import { initSessionSelection } from "./stores/sessions";
import { initLastUsage } from "./composables/useEngine";
import { checkForUpdate } from "./composables/useUpdateCheck";
import { locale } from "./i18n";

initTheme();
initReadingSize();
document.documentElement.lang = locale.value;

// Mount after settings and window state are loaded, so theme, language, the last session and
// the last previewed file do not flash in one after the other.
async function boot(): Promise<void> {
  await initSettings().catch(() => undefined);
  const ui = await loadUiState();
  initWorkArea(ui.workarea);
  initSessionSelection(ui.activeSession);
  initLastUsage(ui.lastUsage);
}

/**
 * The document must stay at the origin. `overflow: hidden` on html/body prevents scrolling,
 * but a stray `scrollIntoView` or a focus inside a very long conversation can still move an
 * embedded scroll container's root in some webviews; this puts it back immediately.
 */
function pinDocument(): void {
  const root = document.scrollingElement;
  if (!root) return;
  if (root.scrollTop !== 0 || root.scrollLeft !== 0) {
    root.scrollTop = 0;
    root.scrollLeft = 0;
  }
}
window.addEventListener("scroll", pinDocument, { passive: true });

void boot().finally(() => {
  createApp(App).mount("#app");
  pinDocument();
  // Deliberately after the window is up and not awaited: whether a newer release exists is
  // worth knowing, but never worth a slower start. The backend keeps its answer for six hours.
  window.setTimeout(() => void checkForUpdate(), 4000);
});
