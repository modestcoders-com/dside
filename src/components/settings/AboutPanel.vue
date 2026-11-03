<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import Icon from "../Icon.vue";
import DeepSeekWhale from "../DeepSeekWhale.vue";
import { t } from "../../i18n";
import { links } from "../../engine/links";

/**
 * The About page. Version and package list come from the manifests themselves (the Rust side
 * embeds both at compile time), so this page cannot drift away from what is actually built.
 */
interface AboutPackage {
  name: string;
  version: string;
}

interface AboutInfo {
  version: string;
  frontend: AboutPackage[];
  backend: AboutPackage[];
}

const info = ref<AboutInfo | null>(null);
const openPanel = ref<"frontend" | "backend" | null>(null);

const authors = ["Ziggy Shēn", "Hohonu Sōusuǒ", "Modest Coder"];

/** Built-with marks, in the order they carry the app: shell, core, view, bundler. */
const stack = [
  { name: "Tauri", logo: "/logos/tauri.svg" },
  { name: "Rust", logo: "/logos/rust.webp" },
  { name: "Vue 3", logo: "/logos/vue3.webp" },
  { name: "Vite", logo: "/logos/vite.svg" },
];

async function openLink(url: string) {
  try {
    if (/^https?:/i.test(url)) await invoke("open_browser_window", { url });
    else await openUrl(url);
  } catch {
    // nothing sensible to do if even that fails
  }
}

onMounted(async () => {
  try {
    info.value = await invoke<AboutInfo>("about_info");
  } catch {
    info.value = null;
  }
});
</script>

<template>
  <div class="aboutpage">
    <!-- One line for what the app is: name, version, who made it. The tagline gets the next. -->
    <header class="hero">
      <DeepSeekWhale class="hero-whale" :height="46" />
      <div class="hero-body">
        <div class="hero-line">
          <h1 class="name">{{ t("app.name") }}</h1>
          <span class="version">v{{ info?.version ?? "—" }}</span>
          <span class="by">
            <span class="by-label">{{ t("about.authors") }}</span>
            {{ authors.join(" · ") }}
          </span>
        </div>
        <p class="tagline">{{ t("about.tagline") }}</p>
      </div>
    </header>

    <!-- "Built with" as one quiet strip instead of a card of four big logos. -->
    <div class="strip">
      <span class="strip-label">{{ t("about.builtWith") }}</span>
      <span v-for="tool in stack" :key="tool.name" class="tool">
        <img :src="tool.logo" :alt="tool.name" />
        <span>{{ tool.name }}</span>
      </span>
    </div>

    <div class="grid">
      <section class="card facts">
        <div class="fact">
          <span class="label">{{ t("about.license") }}</span>
          <span class="value">{{ t("about.licenseValue") }}</span>
        </div>
        <div class="fact">
          <span class="label">{{ t("about.fonts") }}</span>
          <span class="value">{{ t("about.fontsValue") }}</span>
        </div>
      </section>

      <section class="card">
        <h2 class="section">{{ t("about.packages") }}</h2>
        <button
          class="row link"
          type="button"
          @click="openPanel = openPanel === 'frontend' ? null : 'frontend'"
        >
          <span class="row-text">
            <span class="label">{{ t("about.packagesFrontend") }}</span>
            <span class="desc">{{ t("about.packagesCount", { count: info?.frontend.length ?? 0 }) }}</span>
          </span>
          <Icon :name="openPanel === 'frontend' ? 'chevron-down' : 'chevron-right'" :size="14" />
        </button>
        <ul v-if="openPanel === 'frontend'" class="packages">
          <li v-for="entry in info?.frontend ?? []" :key="entry.name">
            <span class="pkg">{{ entry.name }}</span>
            <span class="pkg-version">{{ entry.version }}</span>
          </li>
        </ul>

        <button
          class="row link"
          type="button"
          @click="openPanel = openPanel === 'backend' ? null : 'backend'"
        >
          <span class="row-text">
            <span class="label">{{ t("about.packagesBackend") }}</span>
            <span class="desc">{{ t("about.packagesCount", { count: info?.backend.length ?? 0 }) }}</span>
          </span>
          <Icon :name="openPanel === 'backend' ? 'chevron-down' : 'chevron-right'" :size="14" />
        </button>
        <ul v-if="openPanel === 'backend'" class="packages">
          <li v-for="entry in info?.backend ?? []" :key="entry.name">
            <span class="pkg">{{ entry.name }}</span>
            <span class="pkg-version">{{ entry.version }}</span>
          </li>
        </ul>
      </section>
    </div>

    <section class="addresses">
      <h2 class="section">{{ t("about.links") }}</h2>
      <div class="chips">
        <button class="chip" type="button" @click="openLink(links.home)">
          <span class="chip-label">{{ t("about.homepage") }}</span>
          <span class="chip-value">{{ links.home.replace(/^https?:\/\//, "") }}</span>
          <Icon name="external-link" :size="13" />
        </button>
        <button class="chip" type="button" @click="openLink(links.github)">
          <span class="chip-label">{{ t("about.source") }}</span>
          <span class="chip-value">{{ links.github.replace(/^https?:\/\//, "") }}</span>
          <Icon name="external-link" :size="13" />
        </button>
        <button class="chip" type="button" @click="openLink(links.mail)">
          <span class="chip-label">{{ t("about.contact") }}</span>
          <span class="chip-value">{{ links.mail.replace("mailto:", "") }}</span>
          <Icon name="external-link" :size="13" />
        </button>
      </div>
    </section>

    <p class="note">{{ t("about.licenses") }}</p>
  </div>
</template>

<style scoped>
/* The scroll container is the settings content, not this page: an element with auto height
   cannot scroll. No max-width either — capping it left a wide empty strip on the right. */
.aboutpage {
  container-type: inline-size;
  padding-bottom: 8px;
}

.hero {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 4px 2px 14px;
}

.hero-whale {
  margin-top: -4px;
}

.hero-body {
  flex: 1;
  min-width: 0;
}

.hero-line {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 10px 14px;
}

.name {
  margin: 0;
  color: var(--text-strong);
  font-size: 22px;
  font-weight: 600;
  letter-spacing: 0.2px;
}

.version {
  padding: 2px 7px;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--bg-panel);
  color: var(--text-muted);
  font-family: var(--mono);
  font-size: 11px;
}

.by {
  margin-left: auto;
  color: var(--text);
  font-size: 12px;
}

.by-label {
  color: var(--text-muted);
}

.tagline {
  margin: 6px 0 0;
  color: var(--text-muted);
  font-size: 13px;
}

/* Built-with: one line, small marks, the names carry the meaning. */
.strip {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px 14px;
  padding: 9px 14px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-panel);
}

.strip-label {
  margin-right: 4px;
  color: var(--text-muted);
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.4px;
  text-transform: uppercase;
}

.tool {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--text);
  font-size: 12px;
}

.tool img {
  width: 16px;
  height: 16px;
  object-fit: contain;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 14px;
  margin-top: 14px;
  align-items: start;
}

.card {
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-panel);
  overflow: hidden;
}

.facts {
  display: flex;
  flex-direction: column;
}

.fact {
  display: flex;
  align-items: baseline;
  gap: 14px;
  padding: 11px 16px;
  border-top: 1px solid var(--border);
}

.fact:first-child {
  border-top: none;
}

.label {
  flex: none;
  width: 84px;
  color: var(--text-muted);
  font-size: 12px;
}

.value {
  color: var(--text);
  font-size: 12.5px;
  line-height: 1.45;
}

.section {
  margin: 0;
  padding: 12px 16px 10px;
  color: var(--text-strong);
  font-size: 12px;
  font-weight: 600;
}

.card > .section {
  border-bottom: 1px solid var(--border);
}

.row.link {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  padding: 11px 16px;
  border: none;
  border-top: 1px solid var(--border);
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
}

.row.link:hover {
  background: var(--bg-hover);
}

.row-text {
  display: flex;
  flex: 1;
  align-items: baseline;
  gap: 10px;
  min-width: 0;
}

.row-text .label {
  width: auto;
  color: var(--text);
  font-size: 12.5px;
}

.desc {
  color: var(--text-muted);
  font-family: var(--mono);
  font-size: 11px;
}

.packages {
  display: flex;
  flex-direction: column;
  gap: 3px;
  margin: 0;
  padding: 6px 16px 12px;
  border-top: 1px solid var(--border);
  list-style: none;
  max-height: 260px;
  overflow-y: auto;
}

.packages li {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  color: var(--text-muted);
  font-family: var(--mono);
  font-size: 11px;
}

.pkg-version {
  flex: none;
}

.addresses {
  margin-top: 14px;
}

.addresses .section {
  padding: 0 2px 8px;
}

.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.chip {
  display: inline-flex;
  align-items: baseline;
  gap: 8px;
  max-width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--bg-panel);
  color: var(--text);
  font-size: 12px;
}

.chip:hover {
  border-color: var(--accent);
}

.chip-label {
  color: var(--text-muted);
  font-size: 11px;
}

.chip-value {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--mono);
  font-size: 11px;
}

.note {
  margin: 16px 2px 0;
  color: var(--text-muted);
  font-size: 11px;
  line-height: 1.5;
}
</style>
