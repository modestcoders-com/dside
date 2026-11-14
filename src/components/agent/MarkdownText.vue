<script setup lang="ts">
import { computed } from "vue";
import { escapeHtml, renderMarkdown } from "../../lib/preview";

const props = defineProps<{
  text: string;
  user?: boolean;
  streaming?: boolean;
  tooltip?: string;
  /** No bubble: the text is part of a card that already has a border (the schedule list). */
  bare?: boolean;
}>();

/**
 * Own component on purpose: the parent list re-renders on every streamed chunk, and a
 * cached computed keeps it from parsing and sanitising *every* finished answer again for
 * each of those chunks. Only the block whose text actually changed is rendered anew.
 */
const html = computed(() =>
  props.user
    ? escapeHtml(props.text).replace(/\n/g, "<br />")
    : renderMarkdown(props.text, true),
);
</script>

<template>
  <div
    class="text"
    :class="{ user, streaming, bare }"
    :title="tooltip"
    v-html="html"
  ></div>
</template>

<style scoped>
.text {
  line-height: 1.4;
  padding: 6px 8px;
  border: 1px solid var(--bubble-border);
  border-radius: var(--radius);
  background-color: var(--bubble-bg);
  font-size: var(--reading-font-size);
  /* Anything a model writes must fit the column: long tokens break, wide blocks scroll
     inside themselves. Without this a markdown table widened the whole list and the chat
     grew a horizontal scrollbar — which is how text ended up outside its bubble. */
  word-break: break-word;
  overflow-wrap: anywhere;
  min-width: 0;
}

.text.user {
  padding: 6px 8px;
  border: 1px solid var(--bubble-border);
  border-radius: var(--radius);
  background-color: var(--bubble-user-bg);
}

/* Inside a card that already draws the frame, the text is only text. */
.text.bare {
  padding: 0;
  border: none;
  background: none;
  font-size: 12px;
  line-height: 1.45;
}

.text :deep(p) {
  margin: 0 0 8px;
}

.text :deep(p:last-child) {
  margin-bottom: 0;
}

/* Code blocks keep their shape and scroll inside themselves; without this a long line widened
   the bubble (the "231 行 …" block did exactly that). */
.text :deep(pre) {
  margin: 8px 0;
  padding: 10px 12px;
  max-width: 100%;
  overflow-x: auto;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg);
  font-family: var(--mono);
  font-size: 11.5px;
  line-height: 1.5;
}

.text :deep(code) {
  padding: 1px 5px;
  border-radius: 4px;
  background: var(--bg-elevated);
  font-family: var(--mono);
  font-size: 0.92em;
  overflow-wrap: anywhere;
}

.text :deep(pre code) {
  padding: 0;
  background: none;
}

.text :deep(ul),
.text :deep(ol) {
  margin: 6px 0;
  padding-left: 20px;
}

.text :deep(a) {
  color: var(--accent);
}

.text :deep(h1),
.text :deep(h2),
.text :deep(h3) {
  margin: 10px 0 6px;
  font-size: 1em;
}

.text :deep(table) {
  display: block;
  max-width: 100%;
  overflow-x: auto;
  border-collapse: collapse;
  font-size: 0.95em;
}

.text :deep(th),
.text :deep(td) {
  padding: 4px 8px;
  border: 1px solid var(--border);
  text-align: left;
  vertical-align: top;
}

.text :deep(th) {
  background: var(--bg-elevated);
  font-weight: 600;
}

.text :deep(img) {
  max-width: 100%;
  height: auto;
}
</style>
