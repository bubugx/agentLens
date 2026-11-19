<script setup lang="ts">
import { computed, ref } from "vue";
import InventoryDetails from "./InventoryDetails.vue";
import { t } from "../lib/i18n";
import { bytes, type Snapshot } from "../lib/types";
import { SlidersHorizontal, RotateCw, ArrowUpRight } from "lucide-vue-next";
const props = withDefaults(defineProps<{ data: Snapshot; controls?: boolean }>(), {
  controls: true,
});
const selectedStatus = ref<string | null>(null);
defineEmits<{ rules: []; refresh: [] }>();
const labels: Record<string, string> = {
  eligible: "Ready for review",
  active: "Agent still running",
  recent: "Within retention window",
  sessions: "Session history protected",
  protected: "Programs / data protected",
  stability: "Waiting for stable observation",
  unsafe: "File safety not verified",
  excluded: "Excluded by cleanup rules",
};
const total = computed(() => props.data.cleanupSummary.reduce((s, b) => s + b.bytes, 0));
const inspecting = computed(() => props.data.scanning && !props.data.scanProgress.background);
</script>
<template>
  <section class="panel cleanup-diagnostics">
    <div class="section-heading">
      <div>
        <h2>{{ t("Cleanup assessment") }}</h2>
        <p
          v-if="inspecting"
          role="status"
        >
          {{ t("Rescanning with current rules…") }}
        </p>
      </div>
      <div
        v-if="controls"
        class="inline-controls"
      >
        <button
          class="button secondary small"
          :disabled="inspecting || !data.monitoring"
          @click="$emit('refresh')"
        >
          <RotateCw
            :size="14"
            :class="{ spin: inspecting }"
          />
          {{ t("Recheck files") }}
        </button>
        <button
          class="button secondary small"
          @click="$emit('rules')"
        >
          <SlidersHorizontal :size="14" />
          {{ t("Adjust cleanup rules") }}
        </button>
      </div>
    </div>
    <div
      v-if="data.cleanupSummary.length"
      class="assessment-buckets"
    >
      <button
        type="button"
        v-for="bucket in data.cleanupSummary"
        :key="bucket.code"
        :class="['assessment-bucket', bucket.code]"
        :aria-label="t('View {0} distribution', t(labels[bucket.code] || bucket.code))"
        @click="selectedStatus = bucket.code"
      >
        <div>
          <i />
          <strong>{{ t(labels[bucket.code] || bucket.code) }}</strong>
          <span>{{ bucket.count.toLocaleString() }} {{ t("files") }}</span>
        </div>
        <b>
          {{ bytes(bucket.bytes) }}
          <ArrowUpRight :size="15" />
        </b>
        <div class="assessment-meter">
          <span
            :style="{
              width: `${Math.max(2, (bucket.bytes / Math.max(1, total)) * 100)}%`,
            }"
          />
        </div>
      </button>
    </div>
    <p
      v-if="!data.scanComplete"
      class="assessment-explanation"
    >
      {{ t("Scan is partial; these counts cover the files successfully inspected.") }}
    </p>
  </section>
  <InventoryDetails
    v-if="selectedStatus"
    :status="selectedStatus"
    :label="t(labels[selectedStatus] || selectedStatus)"
    :scanning="data.scanning"
    @close="selectedStatus = null"
  />
</template>
