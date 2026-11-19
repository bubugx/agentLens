<script setup lang="ts">
import { computed } from "vue";
import { AGENT_META } from "../lib/types";
const props = defineProps<{ id: string; small?: boolean }>();
const meta = computed(
  () =>
    AGENT_META[props.id] || { symbol: "·", color: "#698578", light: "#e8eeeb" },
);
</script>
<template>
  <span
    :class="['agent-icon', `agent-${id}`, { small }]"
    :style="{ '--agent-color': meta.color, '--agent-light': meta.light }"
    aria-hidden="true"
  >
    <svg viewBox="0 0 24 24" fill="none" focusable="false">
      <g v-if="id === 'claude'" stroke="currentColor" stroke-width="1.6" stroke-linecap="round">
        <path d="M12 2.5v19M2.5 12h19M5.3 5.3l13.4 13.4M5.3 18.7 18.7 5.3" />
      </g>
      <g v-else-if="id === 'codex'">
      <path
        d="m12 3 7.8 4.5v9L12 21l-7.8-4.5v-9L12 3Z"
        stroke="currentColor"
        stroke-width="1.6"
      />
      <path
        d="m4.4 7.6 7.6 4.5 7.6-4.5M12 12v8.5M8 5.4l8 4.5v9"
        stroke="currentColor"
        stroke-width="1.5"
      />
      </g>
      <g v-else-if="id === 'cursor'" transform="translate(-0.5 0)">
      <path d="m5 3 15 9-7 2-3 7L5 3Z" fill="currentColor" />
      <path d="m5 3 8 11" stroke="white" stroke-width="1.2" opacity=".7" />
      </g>
      <path v-else-if="id === 'gemini'" d="M12 2C13.3 8.1 15.9 10.7 22 12c-6.1 1.3-8.7 3.9-10 10C10.7 15.9 8.1 13.3 2 12 8.1 10.7 10.7 8.1 12 2Z" fill="currentColor" />
      <path v-else-if="id === 'opencode'" d="M8 8V5a3 3 0 1 0-3 3h14a3 3 0 1 0-3-3v14a3 3 0 1 0 3-3H5a3 3 0 1 0 3 3V8Z" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
      <path v-else-if="id === 'aider'" d="M7.2 7.3C8.1 5.8 10 5 12 5c3.3 0 5 1.8 5 4.7V19m0-8.8h-5.8C8 10.2 6 11.8 6 14.6S8 19 10.8 19c3.5 0 6.2-2.3 6.2-5.6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
      <path v-else-if="id === 'unknown'" d="m12 3 9 9-9 9-9-9 9-9Z" stroke="currentColor" stroke-width="1.7" />
      <text v-else x="12" y="12" dy=".35em" text-anchor="middle" fill="currentColor" font-size="15" font-weight="550">{{ meta.symbol }}</text>
    </svg>
  </span>
</template>
