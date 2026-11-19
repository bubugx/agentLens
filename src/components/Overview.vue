<script setup lang="ts">
import { t } from "../lib/i18n";
import { computed } from "vue";
import {
  Activity,
  ArrowRight,
  ArrowUpRight,
  ChevronRight,
  HardDrive,
  Layers3,
  MemoryStick,
  ShieldCheck,
  Sparkles,
  Trash2,
  TriangleAlert,
  Zap,
} from "lucide-vue-next";
import type { Snapshot, Page } from "../lib/types";
import { AGENT_META, bytes, delta, relativeTime } from "../lib/types";
import AgentIcon from "./AgentIcon.vue";
import AgentTable from "./AgentTable.vue";
import EmptyState from "./EmptyState.vue";
import TrendChart from "./TrendChart.vue";
import ContextHelp from "./ContextHelp.vue";
import InitialScan from "./InitialScan.vue";
const props = defineProps<{ data: Snapshot; preview: boolean }>();
const emit = defineEmits<{ navigate: [page: Page]; agent: [id: string] }>();
const agents = computed(() => props.data.agents.filter((a) => a.installed));
const storage = computed(() => agents.value.reduce((s, a) => s + a.storage, 0));
const memory = computed(() => agents.value.reduce((s, a) => s + a.memory, 0));
const reclaimable = computed(() => agents.value.reduce((s, a) => s + a.reclaimable, 0));
const growth = computed(() => agents.value.reduce((s, a) => s + a.growth, 0));
const active = computed(() => agents.value.filter((a) => a.processes > 0));
const orphans = computed(() => props.data.processes.filter((p) => p.orphan));
const rings = computed(() => {
  let offset = 0;
  return agents.value.map((a) => {
    const portion = a.storage / Math.max(storage.value, 1);
    const r = {
      id: a.id,
      color: AGENT_META[a.id]?.color,
      length: Math.max(0, portion * 427.25 - 4),
      offset: -offset,
    };
    offset += portion * 427.25;
    return r;
  });
});
</script>
<template>
  <div class="page-heading">
    <div>
      <h1>{{ t("Overview") }}</h1>
    </div>
    <div class="heading-status">
      <span :class="['status-orb', { paused: !data.monitoring }]" />
      <div>
        <strong v-if="!data.monitoring || data.scanning">
          {{
            data.monitoring
              ? data.scanning
                ? t("Scanning workspace")
                : t("Everything in sight")
              : t("Monitoring paused")
          }}
        </strong>
        <span>
          {{
            data.scannedAt
              ? t("Last scan {0}", relativeTime(data.scannedAt).toLowerCase())
              : t("Building your first snapshot")
          }}
        </span>
      </div>
    </div>
  </div>
  <InitialScan
    v-if="!data.scannedAt"
    :data="data"
    @navigate="emit('navigate', $event)"
  />
  <template v-else>
    <div class="metrics-grid">
      <button
        class="metric-card featured"
        @click="emit('navigate', 'Storage')"
      >
        <div class="metric-label">
          <HardDrive :size="16" />
          {{ t("Agent storage") }}
          <ArrowUpRight :size="14" />
        </div>
        <div class="metric-value">
          {{ bytes(storage).split(" ")[0] }}
          <span>{{ bytes(storage).split(" ")[1] }}</span>
        </div>
        <div class="metric-bottom">
          <span class="growth-pill">{{ delta(growth) }}</span>
          <span>{{ t("since baseline") }}</span>
        </div>
        <svg
          class="metric-spark"
          viewBox="0 0 112 38"
        >
          <path
            d="M0 34 12 30 22 31 32 22 44 25 55 16 65 19 76 11 87 13 99 4 112 6"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          />
        </svg>
      </button>
      <button
        class="metric-card"
        @click="emit('navigate', 'Processes')"
      >
        <div class="metric-label">
          <MemoryStick :size="16" />
          {{ t("Agent memory") }}
          <ArrowUpRight :size="14" />
        </div>
        <div class="metric-value">
          {{ bytes(memory).split(" ")[0] }}
          <span>{{ bytes(memory).split(" ")[1] }}</span>
        </div>
        <div class="metric-bottom">
          <span class="mini-meter">
            <i
              :style="{
                width: `${Math.min(100, (memory / Math.max(1, data.totalMemory)) * 100)}%`,
              }"
            />
          </span>
          <span>
            {{
              data.totalMemory
                ? t(
                    "{0}% of {1}",
                    ((memory / data.totalMemory) * 100).toFixed(1),
                    bytes(data.totalMemory, 0),
                  )
                : t("Reading system memory")
            }}
          </span>
        </div>
      </button>
      <button
        class="metric-card"
        @click="emit('navigate', 'Cleanup')"
      >
        <div class="metric-label">
          <Sparkles :size="16" />
          {{ t("Ready to reclaim") }}
          <ArrowUpRight :size="14" />
        </div>
        <div class="metric-value">
          {{ bytes(reclaimable).split(" ")[0] }}
          <span>{{ bytes(reclaimable).split(" ")[1] }}</span>
        </div>
        <div class="metric-bottom">
          <span class="subtle-green">
            <ShieldCheck :size="13" />
            {{ t("Review before cleanup") }}
          </span>
        </div>
      </button>
      <button
        class="metric-card"
        @click="emit('navigate', 'Agents')"
      >
        <div class="metric-label">
          <Layers3 :size="16" />
          {{ t("Active agents") }}
          <ArrowUpRight :size="14" />
        </div>
        <div class="metric-value">
          {{ active.length }}
          <span class="out-of">/ {{ agents.length }} {{ t("detected") }}</span>
          <span class="metric-live-dot" />
        </div>
        <div class="metric-bottom">
          <span class="agent-mini-stack">
            <AgentIcon
              v-for="a in active.slice(0, 3)"
              :key="a.id"
              :id="a.id"
              small
            />
          </span>
          <span>{{ data.processes.length }} {{ t("running processes") }}</span>
        </div>
      </button>
    </div>
    <div class="overview-middle">
      <TrendChart
        :history="data.history"
        :preview="preview"
      />
      <section class="panel storage-panel">
        <div class="section-heading">
          <h2>{{ t("Storage by agent") }}</h2>
          <ContextHelp :label="t('Storage measurement')">
            <p>
              {{ t("Logical sizes in monitored directories") }}
            </p>
          </ContextHelp>
        </div>
        <div class="storage-ring">
          <svg viewBox="0 0 180 180">
            <circle
              cx="90"
              cy="90"
              r="68"
              fill="none"
              stroke="#eef1ef"
              stroke-width="19"
            />
            <circle
              v-for="r in rings"
              :key="r.id"
              cx="90"
              cy="90"
              r="68"
              fill="none"
              :stroke="r.color"
              stroke-width="19"
              :stroke-dasharray="`${r.length} 427.25`"
              :stroke-dashoffset="r.offset"
              transform="rotate(-90 90 90)"
            />
          </svg>
          <div class="ring-label">
            <strong>
              {{ bytes(storage).split(" ")[0] }}
              <small>{{ bytes(storage).split(" ")[1] }}</small>
            </strong>
            <span>{{ agents.length }} {{ t("connected agents") }}</span>
          </div>
        </div>
        <div class="storage-legend">
          <button
            v-for="a in agents.slice(0, 4)"
            :key="a.id"
            @click="emit('agent', a.id)"
          >
            <span>
              <i :style="{ background: AGENT_META[a.id]?.color }" />
              {{ t(a.name) }}
            </span>
            <strong>{{ bytes(a.storage) }}</strong>
          </button>
        </div>
      </section>
    </div>
    <div class="overview-bottom">
      <section class="panel agents-panel">
        <div class="section-heading">
          <div>
            <h2>{{ t("Your agents") }}</h2>
          </div>
          <button
            class="text-button"
            @click="emit('navigate', 'Agents')"
          >
            {{ t("View all") }}
            <ChevronRight :size="14" />
          </button>
        </div>
        <AgentTable
          v-if="agents.length"
          :agents="agents.slice(0, 4)"
          @select="emit('agent', $event)"
        />
        <EmptyState
          v-else
          :title="t('A fresh workspace')"
          :description="
            t(
              'Known agent directories and active project roots are discovered automatically. Manual addition is optional.',
            )
          "
        >
          <button
            class="button secondary"
            @click="emit('navigate', 'Settings')"
          >
            {{ t("Manage directories") }}
          </button>
        </EmptyState>
      </section>
      <section class="panel activity-panel">
        <div class="section-heading">
          <h2>{{ t("Recent activity") }}</h2>
          <button
            class="icon-button"
            :aria-label="t('View all activity')"
            @click="emit('navigate', 'Activity')"
          >
            <ArrowUpRight :size="16" />
          </button>
        </div>
        <div
          v-if="data.activity.length"
          class="activity-list"
        >
          <button
            v-for="a in data.activity.slice(0, 3)"
            :key="a.id"
            class="activity-item"
            @click="emit('navigate', a.kind === 'process' ? 'Processes' : 'Activity')"
          >
            <span :class="['activity-symbol', a.kind]">
              <Trash2
                v-if="a.kind === 'cleanup'"
                :size="15"
              />
              <TriangleAlert
                v-else-if="a.kind === 'process'"
                :size="15"
              />
              <ArrowUpRight
                v-else
                :size="16"
              />
            </span>
            <span class="activity-copy">
              <strong>{{ t(a.title) }}</strong>
              <small>
                {{ a.agent === "system" ? "AgentLens" : AGENT_META[a.agent]?.name }}
                <b>·</b>
                {{ relativeTime(a.time) }}
              </small>
              <em
                v-if="a.bytes !== 0"
                :class="{ negative: a.bytes < 0 }"
              >
                {{ delta(a.bytes) }}
              </em>
            </span>
          </button>
        </div>
        <div
          v-else
          class="quiet-activity"
        >
          <Activity :size="24" />
          <strong>{{ t("A quiet moment") }}</strong>
          <p>{{ t("Storage changes and cleanups will appear here.") }}</p>
        </div>
        <button
          class="activity-footer"
          @click="emit('navigate', 'Activity')"
        >
          {{ t("See the whole picture") }}
          <ArrowRight :size="13" />
        </button>
      </section>
    </div>
    <div
      v-if="orphans.length"
      class="insight-bar attention"
    >
      <div class="insight-icon"><Zap :size="18" /></div>
      <div>
        <strong>{{ t("{0} processes need review", orphans.length) }}</strong>
        <p>
          {{ t("Its parent agent has exited, but the process is still running.") }}
        </p>
      </div>
      <button
        class="text-button"
        @click="emit('navigate', 'Processes')"
      >
        {{ t("Review processes") }}
        <ArrowRight :size="14" />
      </button>
    </div>
    <div
      v-if="!data.scanComplete"
      class="page-footnote"
    >
      <span>
        {{ t("Partial scan") }} · {{ data.fileCount.toLocaleString() }} {{ t("files observed") }}
      </span>
    </div>
  </template>
</template>
