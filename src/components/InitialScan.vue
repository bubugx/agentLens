<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  ArrowUpRight,
  Check,
  Code2,
  Database,
  FileText,
  Folder,
  Layers3,
  Pause,
  Terminal,
  TriangleAlert,
} from "lucide-vue-next";
import { t } from "../lib/i18n";
import type { Page, Snapshot } from "../lib/types";

const props = defineProps<{ data: Snapshot }>();
defineEmits<{ navigate: [page: Page] }>();
const stages = [
  { phase: "discovery", label: "Discover directories" },
  { phase: "indexing", label: "Build file index" },
  { phase: "saving", label: "Prepare storage map" },
];
const lastStage = ref(-1);
watch(
  () => props.data.scanProgress.phase,
  (phase) => {
    const index = stages.findIndex((stage) => stage.phase === phase);
    if (index >= 0) lastStage.value = index;
    else if (!phase) lastStage.value = 0;
  },
  { immediate: true },
);
const stageIndex = computed(() =>
  props.data.scanProgress.phase === "complete"
    ? stages.length
    : lastStage.value,
);
const state = computed(() =>
  props.data.scanning ? "running" : props.data.scanProgress.phase || "paused",
);
// File signals are decorative; all counts and progress remain in ScanStatus.
const nodes = [
  { icon: Folder, x: -196, y: -84, delay: -0.4 },
  { icon: Code2, x: 163, y: -124, delay: -2.7 },
  { icon: Database, x: 215, y: 13, delay: -1.2 },
  { icon: Layers3, x: 153, y: 133, delay: -3.3 },
  { icon: FileText, x: -152, y: 137, delay: -1.9 },
  { icon: Terminal, x: -234, y: 26, delay: -3.8 },
];
</script>

<template>
  <section
    class="initial-scan"
    :data-state="state"
    :data-phase="data.scanProgress.phase"
    :aria-busy="data.scanning"
  >
    <div class="initial-scan-copy">
      <h2>{{ t("Finding your existing agent footprint") }}</h2>
      <ol class="scan-stages" :aria-label="t('Scan stages')">
        <li
          v-for="(stage, index) in stages"
          :key="stage.phase"
          :class="{ active: index === stageIndex, done: index < stageIndex }"
          :aria-current="index === stageIndex ? 'step' : undefined"
        >
          <span class="scan-stage-marker">
            <Check v-if="index < stageIndex" :size="13" />
            <Pause
              v-else-if="index === stageIndex && state === 'paused'"
              :size="12"
            />
            <TriangleAlert
              v-else-if="index === stageIndex && state === 'error'"
              :size="12"
            />
            <span v-else>{{ String(index + 1).padStart(2, "0") }}</span>
          </span>
          <span>{{ t(stage.label) }}</span>
          <i
            v-if="index === stageIndex"
            class="scan-stage-pulse"
            aria-hidden="true"
          />
        </li>
      </ol>
      <div class="scan-shortcuts">
        <button class="text-button" @click="$emit('navigate', 'Processes')">
          {{ t("Processes") }}<ArrowUpRight :size="13" />
        </button>
        <button class="text-button" @click="$emit('navigate', 'Settings')">
          {{ t("Settings") }}<ArrowUpRight :size="13" />
        </button>
      </div>
    </div>

    <div class="scan-visual" aria-hidden="true">
      <div class="scan-field">
        <div class="scan-grid" />
        <div class="scan-grid-beam" />
        <svg class="scan-circuits" viewBox="0 0 560 400" fill="none">
          <path
            d="M84 116H144L228 200H280M443 76H404L280 200M495 213H390L377 200H280M433 333H413L280 200M128 337H143L280 200M46 226H137L163 200H280"
          />
          <path
            class="scan-circuit-cross"
            d="M280 20V46M280 354V380M100 200H126M434 200H460"
          />
        </svg>
        <div class="scan-lens-halo" />
        <div class="scan-ripple scan-ripple-one" />
        <div class="scan-ripple scan-ripple-two" />
        <div class="scan-lens-dial" />
        <div class="scan-lens-glass">
          <div class="scan-lens-sweep" />
          <div class="scan-lens-crosshair" />
        </div>
        <div class="scan-lens-orbit scan-lens-orbit-outer"><i /></div>
        <div class="scan-lens-orbit scan-lens-orbit-inner"><i /></div>
        <div class="scan-equator"><i /></div>
        <div
          v-for="(node, index) in nodes"
          :key="index"
          class="scan-signal-path"
          :style="{
            '--signal-x': `${node.x}px`,
            '--signal-y': `${node.y}px`,
            '--signal-delay': `${node.delay}s`,
          }"
        >
          <div class="scan-node"><component :is="node.icon" :size="17" /></div>
          <i class="scan-signal" />
        </div>
        <div class="scan-lens-core">
          <Check v-if="state === 'complete'" :size="36" />
          <Pause v-else-if="state === 'paused'" :size="28" />
          <TriangleAlert v-else-if="state === 'error'" :size="28" />
          <img v-else src="/icon.svg" alt="" width="76" height="76" />
        </div>
        <span class="scan-reticle scan-reticle-tl" />
        <span class="scan-reticle scan-reticle-tr" />
        <span class="scan-reticle scan-reticle-bl" />
        <span class="scan-reticle scan-reticle-br" />
      </div>
    </div>
  </section>
</template>

<style scoped>
.initial-scan {
  --scan-color: var(--accent);
  --scan-soft: color-mix(in srgb, var(--scan-color) 13%, transparent);
  position: relative;
  isolation: isolate;
  display: grid;
  grid-template-columns: minmax(245px, 0.85fr) minmax(0, 1.4fr);
  align-items: center;
  min-height: 412px;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--scan-color) 25%, var(--border));
  border-radius: 14px;
  background:
    radial-gradient(ellipse at 76% 50%, var(--scan-soft), transparent 65%),
    var(--surface);
  box-shadow: 0 8px 44px color-mix(in srgb, var(--scan-color) 4%, transparent);
  animation: scan-scene-enter 500ms cubic-bezier(0.22, 1, 0.36, 1) both;
}
.initial-scan-copy {
  position: relative;
  z-index: 2;
  padding: 40px 0 40px 42px;
}
.initial-scan h2 {
  max-width: 330px;
  color: var(--text);
  font-size: clamp(22px, 2.1vw, 28px);
  font-weight: 580;
  letter-spacing: -0.8px;
  line-height: 1.55;
  text-wrap: balance;
}
.scan-stages {
  display: flex;
  flex-direction: column;
  gap: 0;
  list-style: none;
  padding: 0;
  margin: 28px 0 30px;
}
.scan-stages li {
  position: relative;
  display: flex;
  align-items: center;
  gap: 13px;
  width: fit-content;
  min-height: 45px;
  color: var(--muted);
  font-size: 12px;
  transition: color 240ms ease;
}
.scan-stages li:not(:last-child)::after {
  content: "";
  position: absolute;
  top: 35px;
  left: 13px;
  width: 1px;
  height: 20px;
  background: var(--border);
}
.scan-stages li.done::after {
  background: color-mix(in srgb, var(--scan-color) 42%, var(--border));
}
.scan-stage-marker {
  display: grid;
  place-items: center;
  width: 27px;
  height: 27px;
  flex-shrink: 0;
  border: 1px solid var(--border);
  border-radius: 50%;
  background: var(--surface);
  color: var(--muted);
  font-size: 9px;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-variant-numeric: tabular-nums;
}
.scan-stages li.active {
  color: var(--text);
}
.scan-stages .active .scan-stage-marker,
.scan-stages .done .scan-stage-marker {
  border-color: color-mix(in srgb, var(--scan-color) 50%, var(--border));
  background: var(--scan-soft);
  color: var(--scan-color);
}
.scan-stages .active .scan-stage-marker {
  box-shadow: 0 0 0 4px var(--scan-soft);
}
.scan-stage-pulse {
  width: 4px;
  height: 4px;
  margin-left: 5px;
  border-radius: 50%;
  background: var(--scan-color);
  box-shadow: 0 0 9px var(--scan-color);
  animation: scan-breathe 1.8s ease-in-out infinite;
}
.scan-shortcuts {
  display: flex;
  flex-wrap: wrap;
  gap: 20px;
}
.scan-shortcuts button {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--secondary);
  font-size: 11px;
}
.scan-shortcuts button:hover {
  color: var(--scan-color);
}
.scan-visual {
  position: relative;
  height: 412px;
  min-width: 0;
}
.scan-field {
  position: absolute;
  width: 560px;
  height: 400px;
  top: 50%;
  left: 50%;
  margin: -200px 0 0 -280px;
  color: var(--scan-color);
}
.scan-grid {
  position: absolute;
  inset: -25px -15px;
  background-image:
    linear-gradient(var(--scan-soft) 1px, transparent 1px),
    linear-gradient(90deg, var(--scan-soft) 1px, transparent 1px);
  background-size: 28px 28px;
  mask-image: radial-gradient(ellipse, #000 12%, transparent 67%);
  opacity: 0.8;
  transform: perspective(550px) rotateX(22deg) rotateZ(-12deg);
}
.scan-grid-beam {
  position: absolute;
  inset: 40px 0;
  background: linear-gradient(
    90deg,
    transparent,
    var(--scan-soft),
    transparent
  );
  mask-image: radial-gradient(ellipse, #000 10%, transparent 64%);
  animation: scan-grid-pass 6s ease-in-out infinite;
}
.scan-circuits {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  stroke: var(--scan-color);
  stroke-width: 1;
  opacity: 0.22;
}
.scan-circuit-cross {
  opacity: 0.6;
}
.scan-lens-halo,
.scan-ripple,
.scan-lens-dial,
.scan-lens-glass,
.scan-lens-orbit,
.scan-equator,
.scan-lens-core {
  position: absolute;
  top: 50%;
  left: 50%;
  border-radius: 50%;
}
.scan-lens-halo {
  width: 370px;
  height: 370px;
  margin: -185px;
  background: radial-gradient(circle, var(--scan-soft) 24%, transparent 68%);
  animation: scan-breathe 4s ease-in-out infinite;
}
.scan-ripple {
  width: 260px;
  height: 260px;
  margin: -130px;
  border: 1px solid var(--scan-color);
  opacity: 0;
  animation: scan-wave 5s cubic-bezier(0.22, 1, 0.36, 1) infinite;
}
.scan-ripple-two {
  animation-delay: -2.5s;
}
.scan-lens-dial {
  width: 292px;
  height: 292px;
  margin: -146px;
  background: repeating-conic-gradient(
    from 0deg,
    var(--scan-color) 0deg 0.65deg,
    transparent 0.65deg 6deg
  );
  mask-image: radial-gradient(
    transparent 67%,
    #000 67.5% 69%,
    transparent 69.5%
  );
  opacity: 0.45;
  animation: scan-orbit-turn 80s linear infinite;
}
.scan-lens-glass {
  width: 230px;
  height: 230px;
  margin: -115px;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--scan-color) 26%, transparent);
  background:
    radial-gradient(circle at 35% 24%, var(--scan-soft), transparent 60%),
    radial-gradient(circle, transparent 62%, var(--scan-soft));
  box-shadow:
    inset 0 0 30px var(--scan-soft),
    0 0 28px var(--scan-soft);
}
.scan-lens-sweep {
  position: absolute;
  inset: 0;
  background: conic-gradient(
    from 0deg,
    transparent 0deg 270deg,
    color-mix(in srgb, var(--scan-color) 4%, transparent) 290deg,
    color-mix(in srgb, var(--scan-color) 26%, transparent) 359deg,
    transparent 360deg
  );
  animation: scan-orbit-turn 4s linear infinite;
}
.initial-scan[data-phase="indexing"] .scan-lens-sweep {
  animation-duration: 3.2s;
}
.initial-scan[data-phase="indexing"] .scan-signal {
  animation-duration: 3s;
}
.initial-scan[data-phase="saving"] .scan-lens-orbit-outer {
  animation-duration: 8s;
}
.initial-scan[data-phase="saving"] .scan-node {
  opacity: 0.45;
}
.scan-lens-crosshair {
  position: absolute;
  inset: 15px;
  border-radius: 50%;
  border: 1px solid var(--scan-soft);
  background:
    linear-gradient(
      transparent calc(50% - 0.5px),
      var(--scan-soft) 50%,
      transparent calc(50% + 0.5px)
    ),
    linear-gradient(
      90deg,
      transparent calc(50% - 0.5px),
      var(--scan-soft) 50%,
      transparent calc(50% + 0.5px)
    );
}
.scan-lens-orbit {
  border: 1px solid var(--scan-soft);
  animation: scan-orbit-turn 18s linear infinite;
}
.scan-lens-orbit-outer {
  width: 318px;
  height: 318px;
  margin: -159px;
  border-top-color: color-mix(in srgb, var(--scan-color) 65%, transparent);
  border-right-color: color-mix(in srgb, var(--scan-color) 35%, transparent);
}
.scan-lens-orbit-inner {
  width: 254px;
  height: 254px;
  margin: -127px;
  animation-direction: reverse;
  animation-duration: 12s;
  border-left-color: color-mix(in srgb, var(--scan-color) 45%, transparent);
}
.scan-lens-orbit i,
.scan-equator i {
  position: absolute;
  width: 5px;
  height: 5px;
  left: 50%;
  top: -3px;
  border-radius: 50%;
  background: var(--scan-color);
  box-shadow: 0 0 12px var(--scan-color);
}
.scan-equator {
  width: 360px;
  height: 144px;
  margin: -72px -180px;
  border: 1px solid color-mix(in srgb, var(--scan-color) 26%, transparent);
  transform: rotate(-28deg);
}
.scan-equator i {
  top: 50%;
  left: -3px;
  width: 4px;
  height: 4px;
}
.scan-signal-path {
  position: absolute;
  top: 50%;
  left: 50%;
}
.scan-node {
  position: absolute;
  display: grid;
  place-items: center;
  width: 36px;
  height: 36px;
  margin: -18px;
  border: 1px solid color-mix(in srgb, var(--scan-color) 24%, var(--border));
  border-radius: 10px;
  background: var(--surface);
  color: color-mix(in srgb, var(--scan-color) 70%, var(--secondary));
  box-shadow: 0 3px 18px var(--scan-soft);
  transform: translate(var(--signal-x), var(--signal-y));
}
.scan-signal {
  position: absolute;
  width: 4px;
  height: 4px;
  margin: -2px;
  border-radius: 50%;
  background: var(--scan-color);
  box-shadow: 0 0 9px var(--scan-color);
  opacity: 0;
  animation: scan-signal-flow 4.2s cubic-bezier(0.4, 0, 0.2, 1) infinite;
  animation-delay: var(--signal-delay);
}
.scan-lens-core {
  display: grid;
  place-items: center;
  width: 100px;
  height: 100px;
  margin: -50px;
  border: 1px solid color-mix(in srgb, var(--scan-color) 38%, transparent);
  background: var(--surface);
  box-shadow:
    0 0 0 9px var(--scan-soft),
    0 0 38px var(--scan-soft);
}
.scan-lens-core img {
  border-radius: 20px;
}
.scan-reticle {
  position: absolute;
  width: 12px;
  height: 12px;
  border-color: color-mix(in srgb, var(--scan-color) 35%, transparent);
  border-style: solid;
  border-width: 0;
}
.scan-reticle-tl {
  top: 30px;
  left: 76px;
  border-top-width: 1px;
  border-left-width: 1px;
}
.scan-reticle-tr {
  top: 30px;
  right: 76px;
  border-top-width: 1px;
  border-right-width: 1px;
}
.scan-reticle-bl {
  bottom: 30px;
  left: 76px;
  border-bottom-width: 1px;
  border-left-width: 1px;
}
.scan-reticle-br {
  bottom: 30px;
  right: 76px;
  border-bottom-width: 1px;
  border-right-width: 1px;
}
.initial-scan[data-state="error"] {
  --scan-color: var(--danger);
}
.initial-scan[data-state="complete"] {
  --scan-color: var(--success);
}
.initial-scan:not([data-state="running"]) *,
.initial-scan:not([data-state="running"]) *::before,
.initial-scan:not([data-state="running"]) *::after {
  animation-play-state: paused;
}
.initial-scan:not([data-state="running"]) .scan-lens-sweep,
.initial-scan:not([data-state="running"]) .scan-signal,
.initial-scan:not([data-state="running"]) .scan-ripple,
.initial-scan:not([data-state="running"]) .scan-stage-pulse {
  visibility: hidden;
}
@keyframes scan-scene-enter {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
@keyframes scan-orbit-turn {
  to {
    transform: rotate(360deg);
  }
}
@keyframes scan-breathe {
  0%,
  100% {
    opacity: 0.45;
  }
  50% {
    opacity: 1;
  }
}
@keyframes scan-wave {
  0% {
    transform: scale(0.78);
    opacity: 0;
  }
  16% {
    opacity: 0.2;
  }
  100% {
    transform: scale(1.46);
    opacity: 0;
  }
}
@keyframes scan-signal-flow {
  0% {
    transform: translate(var(--signal-x), var(--signal-y));
    opacity: 0;
  }
  15% {
    opacity: 0.8;
  }
  72%,
  100% {
    transform: translate(0, 0);
    opacity: 0;
  }
}
@keyframes scan-grid-pass {
  0%,
  100% {
    transform: translateX(-100px);
    opacity: 0;
  }
  45% {
    opacity: 0.7;
  }
  80% {
    transform: translateX(100px);
    opacity: 0;
  }
}
@media (max-width: 1150px) {
  .initial-scan {
    grid-template-columns: minmax(230px, 0.95fr) minmax(0, 1.3fr);
    min-height: 380px;
  }
  .initial-scan-copy {
    padding-left: 28px;
  }
  .initial-scan h2 {
    font-size: 23px;
  }
  .scan-visual {
    height: 380px;
  }
  .scan-field {
    transform: scale(0.82);
  }
}
@media (max-width: 850px) {
  .initial-scan {
    grid-template-columns: minmax(200px, 1fr) minmax(0, 1fr);
  }
  .initial-scan-copy {
    padding-left: 22px;
  }
  .initial-scan h2 {
    font-size: 21px;
  }
  .scan-field {
    transform: scale(0.68);
  }
}
@media (prefers-reduced-motion: reduce) {
  .initial-scan,
  .initial-scan * {
    animation: none;
    transition: none;
  }
  .scan-signal,
  .scan-ripple,
  .scan-grid-beam {
    display: none;
  }
  .scan-lens-sweep {
    transform: rotate(-35deg);
    opacity: 0.4;
  }
}
</style>
