<script setup lang="ts">
import { computed } from "vue";
import { t } from "../lib/i18n";
import { AGENT_META, bytes, type Agent } from "../lib/types";
const props = defineProps<{ agents: Agent[] }>();
defineEmits<{ select: [id: string] }>();
const sorted = computed(() =>
  props.agents
    .filter((a) => a.installed || a.storage > 0)
    .slice()
    .sort((a, b) => b.storage - a.storage),
);
const total = computed(() => sorted.value.reduce((s, a) => s + a.storage, 0));
const percent = (size: number) => (total.value > 0 ? (size / total.value) * 100 : 0);
const percentText = (size: number) =>
  size === 0 ? "0%" : percent(size) < 0.01 ? "<0.01%" : `${percent(size).toFixed(2)}%`;
</script>
<template>
  <div
    class="agent-storage-breakdown"
    :aria-label="t('Storage distribution by Agent')"
  >
    <div class="agent-storage-heading">
      <span>{{ t("Storage distribution") }}</span>
      <small>{{ t("Largest first") }}</small>
    </div>
    <button
      v-for="agent in sorted"
      :key="agent.id"
      class="agent-storage-row"
      @click="$emit('select', agent.id)"
    >
      <span class="agent-storage-name">
        <i
          :style="{
            background: AGENT_META[agent.id]?.color || 'var(--accent)',
          }"
        />
        {{ t(agent.name) }}
      </span>
      <span class="agent-storage-track">
        <i
          v-if="agent.storage > 0"
          :style="{
            width: `${percent(agent.storage)}%`,
            background: AGENT_META[agent.id]?.color || 'var(--accent)',
          }"
        />
      </span>
      <strong>{{ bytes(agent.storage) }}</strong>
      <small>{{ percentText(agent.storage) }}</small>
    </button>
  </div>
</template>
