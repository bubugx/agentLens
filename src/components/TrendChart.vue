<script setup lang="ts">
import { t } from "../lib/i18n";
import UiTabs from "./UiTabs.vue";
import UiSelect from "./UiSelect.vue";
import { computed, ref, useId } from "vue";
import { TrendingUp } from "lucide-vue-next";
import type { HistoryPoint } from "../lib/types";
import { AGENT_META, bytes } from "../lib/types";
const props = defineProps<{ history: HistoryPoint[]; preview?: boolean }>();
type Metric = "storage" | "memory" | "cpu";
const ranges = [
  { label: "1H", seconds: 3600 },
  { label: "24H", seconds: 86400 },
  { label: "7D", seconds: 604800 },
  { label: "30D", seconds: 2592000 },
];
const range = ref("24H"),
  metric = ref<Metric>("storage"),
  hover = ref<number | null>(null),
  hidden = ref(new Set<string>()),
  id = useId();
const W = 800,
  H = 182,
  left = 44,
  right = 12,
  top = 10,
  bottom = 25;
const points = computed(() =>
  props.history.filter(
    (p) => p.time >= Date.now() / 1000 - ranges.find((r) => r.label === range.value)!.seconds,
  ),
);
const agentIds = computed(() =>
  [...new Set(points.value.flatMap((p) => p.agents.map(([a]) => a)))].filter((a) =>
    points.value.some((p) => p.agents.some(([id, v]) => id === a && v > 0)),
  ),
);
const series = computed(() =>
  metric.value === "storage"
    ? agentIds.value
        .filter((a) => !hidden.value.has(a))
        .map((a) => ({
          id: a,
          color: AGENT_META[a]?.color || "#548f80",
          values: points.value.map((p) => p.agents.find(([id]) => id === a)?.[1] || 0),
        }))
    : [
        {
          id: metric.value,
          color: "#659c8a",
          values: points.value.map((p) => p[metric.value]),
        },
      ],
);
const totals = computed(() =>
  points.value.map((_, i) => series.value.reduce((s, a) => s + a.values[i], 0)),
);
const max = computed(() => {
  const peak = Math.max(...totals.value, metric.value === "cpu" ? 10 : 1e9);
  const unit = metric.value === "cpu" ? 10 : peak >= 1e9 ? 5e9 : 1e8;
  return Math.ceil((peak * 1.12) / unit) * unit;
});
const x = (i: number) => left + (i / Math.max(points.value.length - 1, 1)) * (W - left - right);
const y = (v: number) => H - bottom - (v / max.value) * (H - bottom - top);
const line = (pts: [number, number][]) =>
  pts.map(([x, y], i) => `${i ? "L" : "M"}${x.toFixed(2)},${y.toFixed(2)}`).join(" ");
const areas = computed(() => {
  let base = points.value.map(() => 0);
  return series.value.map((s) => {
    const lower = base.map((v, i) => [x(i), y(v)] as [number, number]);
    base = base.map((v, i) => v + s.values[i]);
    const upper = base.map((v, i) => [x(i), y(v)] as [number, number]);
    return {
      ...s,
      path: `${line(upper)} ${line([...lower].reverse()).replace("M", "L")} Z`,
      stroke: line(upper),
    };
  });
});
const start = computed(() => points.value[0]?.time || Date.now() / 1000 - 86400),
  end = computed(() => points.value.at(-1)?.time || Date.now() / 1000);
const selected = computed(() => (hover.value === null ? null : points.value[hover.value]));
function move(e: MouseEvent) {
  const bounds = (e.currentTarget as SVGElement).getBoundingClientRect();
  const pos = (((e.clientX - bounds.left) / bounds.width) * W - left) / (W - left - right);
  hover.value = Math.max(
    0,
    Math.min(points.value.length - 1, Math.round(pos * (points.value.length - 1))),
  );
}
function toggle(a: string) {
  if (metric.value !== "storage") return;
  const next = new Set(hidden.value);
  if (next.has(a)) next.delete(a);
  else next.add(a);
  hidden.value = next;
}
const time = (t: number) =>
  new Date(t * 1000).toLocaleTimeString("en", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
</script>
<template>
  <section class="panel chart-panel">
    <div class="chart-heading">
      <div>
        <h2>
          {{ t("Resource trends") }}
        </h2>
      </div>
      <UiTabs
        v-model="range"
        class="segmented"
        compact
        :label="t('Trend time range')"
        :options="ranges.map((r) => ({ value: r.label, label: r.label }))"
        @update:model-value="hover = null"
      />
    </div>
    <div class="chart-meta">
      <label class="metric-select">
        <UiSelect
          v-model="metric"
          :label="t('Chart metric')"
          compact
          :options="[
            { value: 'storage', label: t('Storage usage') },
            { value: 'memory', label: t('Agent memory') },
            { value: 'cpu', label: t('Agent CPU') },
          ]"
        />
      </label>
      <span class="chart-total">
        {{ metric === "cpu" ? `${(totals.at(-1) || 0).toFixed(1)}%` : bytes(totals.at(-1) || 0) }}
        <span v-if="points.length < 2">{{ t("building baseline") }}</span>
      </span>
    </div>
    <div class="chart-canvas">
      <div
        v-if="points.length < 2"
        class="chart-wait"
      >
        <TrendingUp :size="28" />
        <strong>{{ t("Your history starts here") }}</strong>
        <span>
          {{
            points.length === 1
              ? t("The next sample arrives within a minute.")
              : t("No samples in this time range. Try a wider range.")
          }}
        </span>
      </div>
      <svg
        v-else
        :viewBox="`0 0 ${W} ${H}`"
        preserveAspectRatio="none"
        :aria-label="t('{0} history chart', metric)"
        @mouseleave="hover = null"
        @mousemove="move"
      >
        <defs>
          <linearGradient
            v-for="a in areas"
            :id="`${id}-${a.id}`"
            :key="a.id"
            x1="0"
            y1="0"
            x2="0"
            y2="1"
          >
            <stop
              :stop-color="a.color"
              stop-opacity=".36"
            />
            <stop
              offset="1"
              :stop-color="a.color"
              stop-opacity=".12"
            />
          </linearGradient>
        </defs>
        <g
          v-for="i in [0, 1, 2, 3]"
          :key="i"
        >
          <line
            :x1="left"
            :y1="y((max * i) / 3)"
            :x2="W - right"
            :y2="y((max * i) / 3)"
            stroke="#e9eeeb"
            stroke-dasharray="3 4"
          />
          <text
            :x="left - 10"
            :y="y((max * i) / 3) + 3"
            text-anchor="end"
            class="chart-axis"
          >
            {{
              metric === "cpu"
                ? `${Math.round((max * i) / 3)}%`
                : `${Math.round((max * i) / 3 / (max >= 1e9 ? 1e9 : 1e6))} ${max >= 1e9 ? "GB" : "MB"}`
            }}
          </text>
        </g>
        <g
          v-for="a in areas"
          :key="a.id"
        >
          <path
            :d="a.path"
            :fill="`url(#${id}-${a.id})`"
          />
          <path
            :d="a.stroke"
            fill="none"
            :stroke="a.color"
            stroke-width="1.7"
            stroke-linejoin="round"
          />
        </g>
        <text
          v-for="(t, i) in [0, 0.25, 0.5, 0.75, 1]"
          :key="t"
          :x="left + t * (W - left - right)"
          :y="H - 3"
          :text-anchor="i === 0 ? 'start' : i === 4 ? 'end' : 'middle'"
          class="chart-axis"
        >
          {{ time(start + t * (end - start)) }}
        </text>
        <g v-if="hover !== null">
          <line
            :x1="x(hover)"
            :x2="x(hover)"
            :y1="top"
            :y2="H - bottom"
            stroke="#627e70"
            stroke-dasharray="3 3"
          />
          <circle
            :cx="x(hover)"
            :cy="y(totals[hover] || 0)"
            r="3.5"
            fill="#406d5c"
            stroke="white"
            stroke-width="2"
          />
        </g>
      </svg>
      <div
        v-if="selected && hover !== null"
        class="chart-tooltip"
        :style="{
          left: `${Math.max(12, Math.min(77, (x(hover) / W) * 100))}%`,
        }"
      >
        <strong>{{ time(selected.time) }}</strong>
        <span>
          {{ metric === "cpu" ? t("{0}% CPU", totals[hover].toFixed(1)) : bytes(totals[hover]) }}
        </span>
      </div>
    </div>
    <div class="chart-legend">
      <button
        v-for="a in metric === 'storage' ? agentIds : [metric]"
        :key="a"
        :class="{ 'disabled-legend': hidden.has(a) }"
        @click="toggle(a)"
      >
        <i :style="{ background: AGENT_META[a]?.color || '#659c8a' }" />
        {{ AGENT_META[a]?.name || (a === "memory" ? t("Resident memory") : t("CPU utilization")) }}
      </button>
      <span class="chart-footnote">
        {{
          metric === "storage"
            ? t("Logical file size")
            : metric === "memory"
              ? t("Sum of readable RSS")
              : t("100% = one CPU core")
        }}
      </span>
    </div>
  </section>
</template>
