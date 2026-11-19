<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import {
  CheckCheck,
  LoaderCircle,
  Pause,
  Play,
  X,
  RotateCw,
  FolderSearch,
  TriangleAlert,
} from "lucide-vue-next";
import { bytes, compactPath, type Snapshot } from "../lib/types";
import { t } from "../lib/i18n";
const props = defineProps<{ data: Snapshot }>();
defineEmits<{ toggle: []; rescan: [] }>();
const dismissed = ref(false);
const progress = computed(() => props.data.scanProgress);
// Folder permission prompts can block filesystem calls without new progress events.
// Keep elapsed time live, while leaving actual scan counters untouched.
const clock = ref(performance.now());
const receivedAt = ref(clock.value);
const lastChangeAt = ref(clock.value);
const timer = window.setInterval(() => (clock.value = performance.now()), 1000);
onBeforeUnmount(() => window.clearInterval(timer));
watch(
  [
    () => progress.value.phase,
    () => progress.value.elapsedMs,
    () => props.data.scanning,
  ],
  () => (receivedAt.value = performance.now()),
);
watch(
  [
    () => progress.value.phase,
    () => progress.value.directories,
    () => progress.value.files,
    () => progress.value.rootsDone,
    () => props.data.scanning,
  ],
  () => (lastChangeAt.value = performance.now()),
);
const elapsedSeconds = computed(() =>
  Math.floor(
    (progress.value.elapsedMs +
      (props.data.scanning ? Math.max(0, clock.value - receivedAt.value) : 0)) /
      1000,
  ),
);
const waitingForAccess = computed(
  () =>
    props.data.scanning &&
    ["discovery", "indexing"].includes(progress.value.phase) &&
    clock.value - lastChangeAt.value >= 8000,
);
const partial = computed(
  () => progress.value.phase === "complete" && !props.data.scanComplete,
);
const visible = computed(
  () =>
    !dismissed.value &&
    ((!progress.value.background && props.data.scanning) ||
      ["paused", "error"].includes(progress.value.phase) ||
      (progress.value.initial && progress.value.phase === "complete")),
);
watch(
  () => props.data.scanning,
  (active) => {
    if (active) dismissed.value = false;
  },
);
const title = computed(() =>
  progress.value.phase === "error"
    ? t("Scan needs another try")
    : progress.value.phase === "paused"
      ? t("Inventory scan paused")
      : partial.value
        ? t("Inventory ready · some folders unavailable")
        : progress.value.phase === "complete"
          ? t("Initial inventory ready")
          : progress.value.initial
            ? t("Building your first inventory")
            : t("Updating file inventory"),
);
const phase = computed(() =>
  partial.value
    ? t("Accessible files are included; review folder access issues below")
    : t(
        (
          {
            discovery: "Discovering projects in your user space",
            indexing: "Inspecting files and session metadata",
            saving: "Saving inventory and preparing distribution",
            complete: "Existing files are now included in monitoring",
            paused: "Resume monitoring to continue the inventory",
            error: "Your previous inventory is still available",
            "": "Preparing the scan…",
          } as Record<string, string>
        )[progress.value.phase],
      ),
);
const percent = computed(() =>
  progress.value.rootsTotal
    ? Math.max(
        0,
        Math.min(
          100,
          Math.round(
            (progress.value.rootsDone / progress.value.rootsTotal) * 100,
          ),
        ),
      )
    : 0,
);
const determinate = computed(
  () => progress.value.phase === "indexing" && progress.value.rootsTotal > 0,
);
</script>
<template>
  <section
    v-if="visible"
    :class="[
      'scan-status',
      { complete: progress.phase === 'complete', scanning: data.scanning },
    ]"
    :data-phase="progress.phase"
    :aria-busy="data.scanning"
  >
    <div class="scan-status-icon">
      <CheckCheck v-if="progress.phase === 'complete'" :size="21" /><Pause
        v-else-if="progress.phase === 'paused'"
        :size="19"
      />
      <TriangleAlert v-else-if="progress.phase === 'error'" :size="20" />
      <FolderSearch
        v-else-if="progress.phase !== 'saving'"
        :size="21"
      /><LoaderCircle v-else :size="21" :class="{ spin: data.scanning }" />
    </div>
    <div class="scan-status-main">
      <div class="scan-status-heading">
        <strong>{{ title }}</strong><span>{{ phase }}</span
        ><small>{{ t("{0}s elapsed", elapsedSeconds) }}</small>
      </div>
      <div
        v-if="data.scanning"
        :class="['scan-track', { indeterminate: !determinate }]"
        role="progressbar"
        :aria-label="t('File inventory progress')"
        :aria-valuenow="determinate ? percent : undefined"
        :aria-valuemin="0"
        :aria-valuemax="100"
      >
        <span
          :style="{
            transform: determinate ? `scaleX(${percent / 100})` : undefined,
          }"
        />
      </div>
      <div class="scan-status-stats">
        <span>{{
          t("{0} directories checked", progress.directories.toLocaleString())
        }}</span><span>{{
          t("{0} files indexed", progress.files.toLocaleString())
        }}</span
        ><span>{{ bytes(progress.bytes) }}</span
        ><span v-if="progress.projects">{{
          t("{0} projects found", progress.projects)
        }}</span
        ><span v-if="progress.rootsTotal">{{
          t("{0}/{1} roots finished", progress.rootsDone, progress.rootsTotal)
        }}</span
        ><span v-if="progress.workers">{{
          t("{0} parallel workers", progress.workers)
        }}</span
        ><span v-if="progress.errors">{{
          t("{0} access issues", progress.errors)
        }}</span>
      </div>
      <p
        v-if="data.scanning && progress.currentPath"
        class="scan-current-path"
        :title="progress.currentPath"
      >
        {{ compactPath(progress.currentPath) }}
      </p>
      <p v-if="waitingForAccess" class="scan-history-note" role="status">
        {{
          t(
            "This directory is taking longer to access. If your system asks for folder access, choose whether to allow it. Denied folders will be skipped.",
          )
        }}
      </p>
      <p v-if="progress.initial" class="scan-history-note">
        {{
          t(
            "Existing agent files become your storage baseline. Memory and CPU history starts with live observations.",
          )
        }}
      </p>
    </div>
    <div class="scan-status-actions">
      <button
        v-if="data.scanning"
        class="button secondary small"
        @click="$emit('toggle')"
      >
        <Pause :size="13" />{{ t("Pause") }}</button
      ><button
        v-else-if="progress.phase === 'paused' && !data.monitoring"
        class="button secondary small"
        @click="$emit('toggle')"
      >
        <Play :size="13" />{{ t("Resume") }}</button
      ><button
        v-else-if="progress.phase === 'error' || progress.phase === 'paused'"
        class="button secondary small"
        @click="$emit('rescan')"
      >
        <RotateCw :size="13" />{{ t("Try again") }}</button
      ><button
        v-if="!data.scanning"
        class="icon-button"
        :aria-label="t('Dismiss scan status')"
        @click="dismissed = true"
      >
        <X :size="14" />
      </button>
    </div>
  </section>
</template>

<style scoped>
.scan-status {
  background:
    linear-gradient(
      110deg,
      color-mix(in srgb, var(--accent) 4%, transparent),
      transparent 60%
    ),
    var(--surface);
}
.scan-status-icon {
  position: relative;
  width: 40px;
  height: 40px;
  border-radius: 50%;
}
.scan-status-icon::before {
  content: "";
  position: absolute;
  inset: -4px;
  border: 1px solid color-mix(in srgb, var(--accent) 18%, transparent);
  border-top-color: var(--accent);
  border-right-color: var(--accent);
  border-radius: 50%;
  opacity: 0;
}
.scanning .scan-status-icon::before {
  opacity: 1;
  animation: scan-indicator-orbit 2.4s linear infinite;
}
.complete .scan-status-icon {
  color: var(--success);
  background: color-mix(in srgb, var(--success) 9%, var(--surface));
}
.scan-status[data-phase="error"] .scan-status-icon {
  color: var(--danger);
}
.scan-track > span {
  width: 100%;
  min-width: 0;
  transform-origin: left center;
  transition: transform 320ms cubic-bezier(0.22, 1, 0.36, 1);
  background: linear-gradient(
    90deg,
    color-mix(in srgb, var(--accent) 65%, var(--surface)),
    var(--accent)
  );
}
.scan-track.indeterminate > span {
  width: 24%;
}
@keyframes scan-indicator-orbit {
  to {
    transform: rotate(360deg);
  }
}
@media (prefers-reduced-motion: reduce) {
  .scan-status-icon::before {
    animation: none;
  }
  .scan-track > span {
    transition: none;
  }
}
</style>
