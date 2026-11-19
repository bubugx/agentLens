<script setup lang="ts">
import { t } from "../lib/i18n";
import { ChevronRight } from "lucide-vue-next";
import AgentIcon from "./AgentIcon.vue";
import Badge from "./Badge.vue";
import type { Agent } from "../lib/types";
import { bytes, delta } from "../lib/types";
defineProps<{ agents: Agent[]; compact?: boolean }>();
const emit = defineEmits<{ select: [id: string] }>();
</script>
<template>
  <div class="table-wrap">
    <table class="agent-table">
      <thead>
        <tr>
          <th>{{ t("Agent") }}</th>
          <th>{{ t("Status") }}</th>
          <th>{{ t("Storage") }}</th>
          <th>{{ t("Memory") }}</th>
          <th v-if="!compact">
            {{ t("Growth") }} <span class="muted">↗</span>
          </th>
          <th :aria-label="t('Actions')" />
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="agent in agents"
          :key="agent.id"
          tabindex="0"
          @click="emit('select', agent.id)"
          @keydown.enter="emit('select', agent.id)"
        >
          <td>
            <div class="agent-name">
              <AgentIcon :id="agent.id" /><span
                ><strong>{{ t(agent.name) }}</strong
                ><small
                  >{{ agent.sessions }} {{ t("sessions") }} <span>·</span>
                  {{ agent.files.toLocaleString() }} {{ t("files") }}</small
                ></span
              >
            </div>
          </td>
          <td>
            <Badge dot :tone="agent.processes ? 'green' : 'neutral'">{{
              agent.processes ? t("Active") : t("Idle")
            }}</Badge>
          </td>
          <td class="tabular">{{ bytes(agent.storage) }}</td>
          <td class="tabular secondary">
            {{ agent.processes ? bytes(agent.memory) : "—" }}
          </td>
          <td v-if="!compact" class="tabular growth-text">
            {{ delta(agent.growth) }}
          </td>
          <td>
            <button
              class="icon-button"
              :aria-label="t('View {0}', agent.name)"
              @click.stop="emit('select', agent.id)"
            >
              <ChevronRight :size="18" />
            </button>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
