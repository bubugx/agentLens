<script setup lang="ts">
import { computed, ref } from "vue";
import { t, locale } from "../lib/i18n";
import { bytes, delta } from "../lib/types";
import type { ResourcePoint } from "../lib/resources";
const props = defineProps<{
  points: ResourcePoint[];
  metric: "cpu" | "memory";
  loading?: boolean;
}>();
const hover = ref<number | null>(null),
  W = 600,
  H = 164,
  L = 62,
  R = 12,
  T = 15,
  B = 28;
const max = computed(
  () =>
    Math.max(
      props.metric === "cpu" ? 1 : 1e6,
      ...props.points.map((p) => p[props.metric]),
    ) * 1.12,
);
const x = (time: number) =>
  L +
  ((time - (props.points[0]?.time || 0)) /
    Math.max(
      1,
      (props.points.at(-1)?.time || 0) - (props.points[0]?.time || 0),
    )) *
    (W - L - R);
const y = (value: number) => H - B - (value / max.value) * (H - T - B);
const line = computed(() =>
  props.points
    .map((p, i) => `${i ? "L" : "M"}${x(p.time)},${y(p[props.metric])}`)
    .join(" "),
);
const latest = computed(() => props.points.at(-1)),
  change = computed(() =>
    latest.value
      ? latest.value[props.metric] - (props.points[0]?.[props.metric] || 0)
      : 0,
  );
const selected = computed(() => props.points[hover.value ?? -1]);
const value = (n: number) =>
  props.metric === "memory" ? bytes(n) : `${n.toFixed(1)}%`;
const time = (n: number) =>
  new Date(n * 1000).toLocaleString(locale.value, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
function move(event: PointerEvent) {
  const r = (event.currentTarget as SVGElement).getBoundingClientRect();
  const pos = ((event.clientX - r.left) / r.width) * W;
  hover.value = props.points.reduce(
    (best, p, i) =>
      Math.abs(x(p.time) - pos) < Math.abs(x(props.points[best].time) - pos)
        ? i
        : best,
    0,
  );
}
</script>
<template>
  <div :class="['resource-curve', metric]">
    <div class="resource-curve-heading">
      <div>
        <span>{{
          metric === "cpu" ? t("CPU utilization") : t("Resident memory")
        }}</span><strong>{{ latest ? value(latest[metric]) : "—" }}</strong>
      </div>
      <small
        >{{
          metric === "memory"
            ? delta(change)
            : `${change >= 0 ? "+" : ""}${change.toFixed(1)} ${t("pp")}`
        }}
        · {{ t("Range change") }}</small
      >
    </div>
    <div class="resource-curve-canvas">
      <svg
        v-if="points.length"
        :viewBox="`0 0 ${W} ${H}`"
        preserveAspectRatio="none"
        role="img"
        :aria-label="t('{0} resource trend', metric)"
        @pointermove="move"
        @pointerleave="hover = null"
      >
        <g v-for="v in [0, 0.5, 1]" :key="v">
          <line
            :x1="L"
            :x2="W - R"
            :y1="y(v * max)"
            :y2="y(v * max)"
            class="activity-trend-grid"
          />
          <text
            :x="L - 8"
            :y="y(v * max) + 3"
            text-anchor="end"
            class="chart-axis"
          >
            {{ value(v * max) }}
          </text>
        </g>
        <path
          :d="`${line} L${x(points.at(-1)!.time)},${H - B} L${L},${H - B} Z`"
          class="activity-trend-area"
        />
        <path :d="line" class="activity-trend-line" />
        <circle
          v-if="points.length === 1"
          :cx="L"
          :cy="y(points[0][metric])"
          r="4"
          class="activity-trend-dot"
        />
        <text :x="L" :y="H - 4" class="chart-axis">
          {{ time(points[0].time) }}
        </text>
        <text
          v-if="points.length > 1"
          :x="W - R"
          :y="H - 4"
          text-anchor="end"
          class="chart-axis"
        >
          {{ time(points.at(-1)!.time) }}
        </text>
        <g v-if="selected">
          <line
            :x1="x(selected.time)"
            :x2="x(selected.time)"
            :y1="T"
            :y2="H - B"
            class="activity-trend-cursor"
          />
          <circle
            :cx="x(selected.time)"
            :cy="y(selected[metric])"
            r="4"
            class="activity-trend-dot"
          />
        </g>
      </svg>
      <div v-else class="resource-curve-wait">
        {{
          loading
            ? t("Loading resource history…")
            : t("Resource history starts with the first observed sample.")
        }}
      </div>
      <div v-if="selected" class="activity-trend-tooltip">
        {{ time(selected.time) }} · {{ value(selected[metric]) }}
      </div>
    </div>
    <p>
      {{
        metric === "cpu"
          ? t("100% = one CPU core")
          : t(
              "Sum of readable RSS; shared memory can be counted more than once.",
            )
      }}
    </p>
  </div>
</template>
