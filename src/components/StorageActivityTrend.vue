<script setup lang="ts">
import { computed, ref } from "vue";
import { TrendingUp } from "lucide-vue-next";
import { t, locale } from "../lib/i18n";
import { bytes, delta, type Activity, type HistoryPoint } from "../lib/types";
import { storageSeries } from "../lib/activity";
const props = defineProps<{
  history: HistoryPoint[];
  events: Activity[];
  start: number;
  end: number;
}>();
const series = computed(() => storageSeries(props.history, props.start, props.end, props.events));
const points = computed(() => series.value.points),
  hover = ref<number | null>(null);
const W = 900,
  H = 210,
  L = 76,
  R = 16,
  T = 16,
  B = 30;
const min = computed(() =>
  Math.min(
    ...(series.value.mode === "usage" && points.value.length ? [] : [0]),
    ...points.value.map((p) => p.value),
  ),
);
const max = computed(() => {
  const value = Math.max(
    ...(series.value.mode === "usage" && points.value.length ? [] : [0]),
    ...points.value.map((p) => p.value),
  );
  return value === min.value ? min.value + Math.max(1, value * 0.02) : value;
});
const padding = computed(() => Math.max(1, (max.value - min.value) * 0.12));
const low = computed(() =>
    series.value.mode === "usage"
      ? Math.max(0, min.value - padding.value)
      : min.value < 0
        ? min.value - padding.value
        : 0,
  ),
  high = computed(() => max.value + padding.value);
const startTime = computed(() => points.value[0]?.time || props.start),
  endTime = computed(() => points.value.at(-1)?.time || props.end);
const x = (time: number) =>
  L + ((time - startTime.value) / Math.max(1, endTime.value - startTime.value)) * (W - L - R);
const y = (value: number) =>
  H - B - ((value - low.value) / Math.max(1, high.value - low.value)) * (H - T - B);
const line = computed(() =>
  points.value
    .map((p, i) => `${i ? "L" : "M"}${x(p.time).toFixed(2)},${y(p.value).toFixed(2)}`)
    .join(" "),
);
const area = computed(() =>
  points.value.length
    ? `${line.value} L${x(points.value.at(-1)!.time)},${y(low.value)} L${x(points.value[0].time)},${y(low.value)} Z`
    : "",
);
const selected = computed(() => points.value[hover.value ?? -1]);
const format = (time: number) =>
  new Date(time * 1000).toLocaleString(locale.value, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
function move(event: PointerEvent) {
  const bounds = (event.currentTarget as SVGElement).getBoundingClientRect();
  const time =
    startTime.value +
    ((((event.clientX - bounds.left) / bounds.width) * W - L) / (W - L - R)) *
      (endTime.value - startTime.value);
  hover.value = points.value.reduce(
    (best, p, i) => (Math.abs(p.time - time) < Math.abs(points.value[best].time - time) ? i : best),
    0,
  );
}
function key(event: KeyboardEvent) {
  if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
  event.preventDefault();
  hover.value =
    event.key === "Home"
      ? 0
      : event.key === "End"
        ? points.value.length - 1
        : Math.max(
            0,
            Math.min(
              points.value.length - 1,
              (hover.value ?? 0) + (event.key === "ArrowRight" ? 1 : -1),
            ),
          );
}
</script>
<template>
  <section class="panel activity-trend-panel">
    <div class="section-heading">
      <div>
        <h2>
          {{ series.mode === "usage" ? t("Storage usage trend") : t("Observed net change trend") }}
        </h2>
        <p>
          {{
            series.mode === "usage"
              ? t("Monitored file size over time")
              : t("Cumulative net change from retained storage observations")
          }}
        </p>
      </div>
      <div class="activity-trend-total">
        <strong>
          {{
            series.mode === "usage"
              ? bytes(points.at(-1)?.value || 0)
              : delta(points.at(-1)?.value || 0)
          }}
        </strong>
        <small v-if="points.length > 1">
          {{ t("Range change") }} {{ delta(points.at(-1)!.value - points[0].value) }}
        </small>
      </div>
    </div>
    <div
      v-if="points.length"
      class="activity-trend-canvas"
    >
      <svg
        :viewBox="`0 0 ${W} ${H}`"
        preserveAspectRatio="none"
        role="img"
        tabindex="0"
        :aria-label="t('Storage trend chart')"
        @pointermove="move"
        @pointerleave="hover = null"
        @keydown="key"
        @blur="hover = null"
      >
        <g
          v-for="fraction in [0, 0.5, 1]"
          :key="fraction"
        >
          <line
            :x1="L"
            :x2="W - R"
            :y1="y(low + fraction * (high - low))"
            :y2="y(low + fraction * (high - low))"
            class="activity-trend-grid"
          />
          <text
            :x="L - 10"
            :y="y(low + fraction * (high - low)) + 4"
            text-anchor="end"
            class="chart-axis"
          >
            {{ bytes(low + fraction * (high - low)) }}
          </text>
        </g>
        <path
          :d="area"
          class="activity-trend-area"
        />
        <path
          :d="line"
          class="activity-trend-line"
        />
        <circle
          v-if="points.length === 1"
          :cx="x(points[0].time)"
          :cy="y(points[0].value)"
          r="4"
          class="activity-trend-dot"
        />
        <text
          v-for="(fraction, i) in points.length > 1 ? [0, 0.5, 1] : [0]"
          :key="fraction"
          :x="L + fraction * (W - L - R)"
          :y="H - 5"
          :text-anchor="i === 0 ? 'start' : i === 2 ? 'end' : 'middle'"
          class="chart-axis"
        >
          {{ format(startTime + fraction * (endTime - startTime)) }}
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
            :cy="y(selected.value)"
            r="4"
            class="activity-trend-dot"
          />
        </g>
      </svg>
      <div
        v-if="selected"
        class="activity-trend-tooltip"
        role="status"
      >
        <span>{{ format(selected.time) }}</span>
        <strong>
          {{ series.mode === "usage" ? bytes(selected.value) : delta(selected.value) }}
        </strong>
      </div>
    </div>
    <div
      v-else
      class="activity-trend-wait"
    >
      <TrendingUp :size="24" />
      <strong>{{ t("Waiting for storage samples") }}</strong>
      <span>
        {{
          t(
            "A trend appears after a completed inventory. Change the time range to inspect older samples.",
          )
        }}
      </span>
    </div>
    <div class="activity-trend-caption">
      <span>
        <i />
        {{ series.mode === "usage" ? t("Monitored storage") : t("Observed net change") }}
      </span>
      <small>
        {{
          points.length === 1
            ? t("One sample recorded; the next sample will establish a trend.")
            : t("Hover or use arrow keys to inspect samples.")
        }}
      </small>
    </div>
  </section>
</template>
