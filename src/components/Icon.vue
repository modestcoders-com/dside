<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    name: string;
    size?: number;
  }>(),
  { size: 16 },
);

const shapes: Record<string, string[]> = {
  plus: ["M12 5v14", "M5 12h14"],
  search: ["M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16z", "M21 21l-4.3-4.3"],
  grid: ["M4 4h7v7H4z", "M13 4h7v7h-7z", "M4 13h7v7H4z", "M13 13h7v7h-7z"],
  clock: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "M12 7v5l3 2"],
  globe: [
    "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
    "M3 12h18",
    "M12 3c2.4 2.6 2.4 15.4 0 18",
    "M12 3c-2.4 2.6-2.4 15.4 0 18",
  ],
  folder: ["M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"],
  panel: ["M4 5h16v14H4z", "M15 5v14"],
  "panel-left": ["M4 5h16v14H4z", "M9 5v14"],
  "chevron-down": ["M6 9l6 6 6-6"],
  "chevron-right": ["M9 6l6 6-6 6"],
  "chevron-up": ["M6 15l6-6 6 6"],
  gear: [
    "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
    "M12 2v3",
    "M12 19v3",
    "M2 12h3",
    "M19 12h3",
    "M4.9 4.9l2.1 2.1",
    "M17 17l2.1 2.1",
    "M19.1 4.9L17 7",
    "M7 17l-2.1 2.1",
  ],
  user: ["M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8z", "M4 21a8 8 0 0 1 16 0"],
  x: ["M6 6l12 12", "M18 6L6 18"],
  send: ["M12 19V5", "M5 12l7-7 7 7"],
  copy: ["M9 9h12v12H9z", "M5 15H3V3h12v2"],
  "arrow-left": ["M19 12H5", "M12 19l-7-7 7-7"],
  "arrow-right": ["M5 12h14", "M12 5l7 7-7 7"],
  // Used by the update chip: a newer release is something to move up to, not to send.
  "arrow-up": ["M12 19V5", "M5 12l7-7 7 7"],
  reload: ["M20 12a8 8 0 1 1-2.4-5.7", "M20 4v4h-4"],
  dots: ["M6 12h.01", "M12 12h.01", "M18 12h.01"],
  doc: ["M6 3h8l4 4v14H6z", "M14 3v4h4"],
  sun: [
    "M12 17a5 5 0 1 0 0-10 5 5 0 0 0 0 10z",
    "M12 1v2",
    "M12 21v2",
    "M4 12H2",
    "M22 12h-2",
    "M5 5L3.5 3.5",
    "M20.5 20.5L19 19",
    "M19 5l1.5-1.5",
    "M3.5 20.5L5 19",
  ],
  moon: ["M20 14.5A8.5 8.5 0 0 1 9.5 4a8.5 8.5 0 1 0 10.5 10.5z"],
  monitor: ["M4 5h16v11H4z", "M9 20h6", "M12 16v4"],
  check: ["M5 13l4 4 10-11"],
  pin: ["M12 17v5", "M8 3h8l-1 7 3 3v2H6v-2l3-3z"],
  pencil: ["M4 20h4L18.5 9.5a2.1 2.1 0 0 0-3-3L5 17v3z", "M13.5 6.5l3 3"],
  archive: ["M3 7h18v3H3z", "M5 10v9h14v-9", "M10 14h4"],
  sparkle: ["M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8z"],
  "external-link": ["M14 4h6v6", "M20 4l-9 9", "M18 14v5H5V6h5"],
  terminal: ["M4 5h16v14H4z", "M8 10l3 3-3 3", "M13 16h4"],
  diff: ["M5 3h14v18H5z", "M9 8h6", "M9 12h6", "M9 16h3", "M15 16h3"],
  git: [
    "M6 3v12",
    "M18 9a9 9 0 0 1-9 9",
    "M18 9a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
    "M6 21a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
  ],
  activity: ["M3 12h4l3-8 4 16 3-8h4"],
  rudder: [
    "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
    "M12 3v18",
    "M3 12h18",
    "M5.6 5.6l12.8 12.8",
    "M18.4 5.6L5.6 18.4",
  ],
  shield: ["M12 3l7 3v6c0 4.5-3 7.5-7 9-4-1.5-7-4.5-7-9V6z"],
  expand: ["M9 4H4v5", "M15 20h5v-5", "M4 4l6 6", "M20 20l-6-6"],
  "sidebar-toggle": ["M4 5h16v14H4z", "M9 5v14", "M13 10l3 2-3 2"],
  /* A robot for "watch what the companion does": a round head, an antenna, two dot eyes and a
     short mouth. Dots instead of slots, because at 13 px a slot reads as a stripe. */
  robot: [
    "M12 4.2v2.6",
    "M12 2.6a1.2 1.2 0 1 0 0 2.4 1.2 1.2 0 0 0 0-2.4",
    "M8 6.8h8a3 3 0 0 1 3 3v6.4a3 3 0 0 1-3 3H8a3 3 0 0 1-3-3V9.8a3 3 0 0 1 3-3z",
    "M5 11.2v3.6",
    "M19 11.2v3.6",
    "M9.4 11.6h.01",
    "M14.6 11.6h.01",
    "M9.8 15.4h4.4",
  ],
  /* Following is off: the same robot with a blank face and a slash. Dropping the eyes rather
     than drawing the slash across them keeps it readable at 14 px. */
  "robot-off": [
    "M12 4.2v2.6",
    "M12 2.6a1.2 1.2 0 1 0 0 2.4 1.2 1.2 0 0 0 0-2.4",
    "M8 6.8h8a3 3 0 0 1 3 3v6.4a3 3 0 0 1-3 3H8a3 3 0 0 1-3-3V9.8a3 3 0 0 1 3-3z",
    "M5 11.2v3.6",
    "M19 11.2v3.6",
    "M3.4 3.4l17.2 17.2",
  ],
  eye: ["M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6-10-6-10-6z", "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z"],
  "eye-off": [
    "M3 3l18 18",
    "M10.6 6.2A9.9 9.9 0 0 1 12 6c6.5 0 10 6 10 6a17 17 0 0 1-3.1 3.7",
    "M6.1 6.3A17 17 0 0 0 2 12s3.5 6 10 6a9.6 9.6 0 0 0 4-.8",
  ],
};

const segments = computed(() => shapes[props.name] ?? []);
</script>

<template>
  <svg
    class="icon"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.7"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path v-for="(segment, index) in segments" :key="index" :d="segment" />
  </svg>
</template>

<style scoped>
.icon {
  display: block;
  flex: none;
}
</style>
