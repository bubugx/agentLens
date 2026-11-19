<script setup lang="ts">
import { computed, ref, shallowRef } from "vue";
import {
  Check,
  LoaderCircle,
  Terminal,
  TriangleAlert,
  X,
} from "lucide-vue-next";
import Modal from "./Modal.vue";
import AgentIcon from "./AgentIcon.vue";
import { t } from "../lib/i18n";
import {
  purposeLabel,
  processKey,
  stopImpact,
  assessmentReason,
} from "../lib/processes";
import { bytes, type Process, type ProcessStopResult } from "../lib/types";
import { stopProcesses } from "../lib/client";

const props = defineProps<{
  processes: Process[];
  preview: boolean;
  suggested?: boolean;
}>();
const emit = defineEmits<{ close: [] }>();
const busy = ref(false);
const result = shallowRef<ProcessStopResult | null>(null);
const memory = computed(() =>
  props.processes.reduce((sum, p) => sum + p.memory, 0),
);
const cpu = computed(() => props.processes.reduce((sum, p) => sum + p.cpu, 0));
const failures = computed(
  () =>
    new Map(
      result.value?.failures.map(({ process, reason }) => [
        processKey(process),
        reason,
      ]),
    ),
);
async function execute() {
  if (busy.value || result.value) return;
  busy.value = true;
  try {
    result.value = await stopProcesses(props.processes, props.suggested);
  } finally {
    busy.value = false;
  }
}
</script>
<template>
  <Modal
    :title="result ? t('Batch process results') : t('Stop selected processes?')"
    :subtitle="
      t('Review {0} selected processes and their impact.', processes.length)
    "
    wide
    @close="!busy && emit('close')"
  >
    <div class="batch-stop-summary" :aria-busy="busy">
      <Terminal :size="20" />
      <div>
        <strong>{{ t("{0} processes selected", processes.length) }}</strong>
        <span
          >{{ t("Combined RSS {0}", bytes(memory)) }} · CPU
          {{ cpu.toFixed(1) }}%</span
        >
      </div>
    </div>
    <div
      v-if="result"
      class="notice"
      :class="{ amber: result.failures.length }"
      role="status"
    >
      <Check v-if="!result.failures.length" :size="18" /><TriangleAlert
        v-else
        :size="18"
      />
      <span>{{
        preview
          ? t(
              "{0} sample processes stopped · {1} failed",
              result.requested.length,
              result.failures.length,
            )
          : t(
              "{0} termination requests sent · {1} failed",
              result.requested.length,
              result.failures.length,
            )
      }}</span>
    </div>
    <div class="batch-stop-list">
      <div
        v-for="process in processes"
        :key="processKey(process)"
        class="batch-stop-item"
      >
        <div class="batch-stop-identity">
          <AgentIcon :id="process.agent" small />
          <strong>{{ process.name }}</strong><span>PID {{ process.pid }}</span>
        </div>
        <template v-if="result">
          <p
            v-if="failures.has(processKey(process))"
            class="batch-stop-failure"
          >
            <X :size="13" />{{ t(failures.get(processKey(process))!) }}
          </p>
          <p v-else class="batch-stop-success">
            <Check :size="13" />{{
              preview ? t("Sample process stopped") : t("Termination requested")
            }}
          </p>
        </template>
        <template v-else>
          <span class="batch-stop-purpose"
            >{{ purposeLabel(process.purpose) }} · {{ bytes(process.memory) }} ·
            CPU {{ process.cpu.toFixed(1) }}%</span
          >
          <p>{{ stopImpact(process.purpose) }}</p>
          <p v-if="suggested" class="batch-suggestion-reason">
            {{ assessmentReason(process) }}
          </p>
        </template>
      </div>
    </div>
    <div v-if="!result" class="notice amber">
      <TriangleAlert :size="18" /><span>{{
        preview
          ? t("Preview only. No real process will be stopped.")
          : t(
              "Unsaved work or a running task may be interrupted. macOS/Linux send SIGTERM; Windows requests process termination.",
            )
      }}</span>
    </div>
    <p v-else-if="!preview" class="batch-stop-note">
      {{ t("Termination requested. The next sample will confirm its state.") }}
    </p>
    <template #footer>
      <button
        v-if="!result"
        class="button secondary"
        :disabled="busy"
        @click="emit('close')"
      >
        {{ t("Keep running") }}
      </button>
      <button
        v-if="!result"
        class="button danger"
        :disabled="busy"
        @click="execute"
      >
        <LoaderCircle v-if="busy" :size="15" class="spin" /><X
          v-else
          :size="15"
        />
        {{
          busy
            ? t("Sending termination requests…")
            : preview
              ? t("Stop sample processes")
              : t("Confirm batch stop")
        }}
      </button>
      <button v-else class="button primary" @click="emit('close')">
        {{ t("Done") }}
      </button>
    </template>
  </Modal>
</template>
