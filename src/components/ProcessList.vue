<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { t } from "../lib/i18n";
import {
  purposeLabel,
  stopImpact,
  processKey,
  assessmentCategory,
  assessmentReason,
} from "../lib/processes";
import {
  AGENT_META,
  bytes,
  delta,
  compactPath,
  type Process,
} from "../lib/types";
import {
  Check,
  Minus,
  ChevronDown,
  ChevronRight,
  Terminal,
  X,
} from "lucide-vue-next";
import AgentIcon from "./AgentIcon.vue";
import Badge from "./Badge.vue";
const props = defineProps<{ processes: Process[]; all: Process[] }>();
defineEmits<{ stop: [process: Process]; stopMany: [processes: Process[]] }>();
const expanded = ref<number | null>(null);
const parent = (p: Process) => props.all.find((v) => v.pid === p.parentPid);
const selected = ref(new Set<string>());
const selectedProcesses = computed(() =>
  props.processes.filter((p) => selected.value.has(processKey(p))),
);
const allSelected = computed(
  () =>
    props.processes.length > 0 &&
    selectedProcesses.value.length === props.processes.length,
);
const selectedMemory = computed(() =>
  selectedProcesses.value.reduce((total, p) => total + p.memory, 0),
);
watch(
  () => props.processes,
  (processes) => {
    const visible = new Set(processes.map(processKey));
    selected.value = new Set(
      [...selected.value].filter((key) => visible.has(key)),
    );
  },
);
function toggle(process: Process) {
  const next = new Set(selected.value);
  const key = processKey(process);
  if (next.has(key)) next.delete(key);
  else next.add(key);
  selected.value = next;
}
function toggleAll() {
  selected.value = allSelected.value
    ? new Set()
    : new Set(props.processes.map(processKey));
}
</script>
<template>
  <div class="process-selection-bar">
    <div class="process-selection-summary" role="status" aria-live="polite">
      <template v-if="selectedProcesses.length">
        <strong>{{
          t("{0} processes selected", selectedProcesses.length)
        }}</strong>
        <span>{{ t("Combined RSS {0}", bytes(selectedMemory)) }}</span>
      </template>
      <span v-else>{{ processes.length }} {{ t("processes") }}</span>
    </div>
    <div class="inline-controls">
      <button
        v-if="selectedProcesses.length"
        class="button secondary small"
        :disabled="!selected.size"
        @click="selected = new Set()"
      >
        {{ t("Clear selection") }}
      </button>
      <button
        class="button danger-ghost small"
        :disabled="!selectedProcesses.length"
        @click="$emit('stopMany', selectedProcesses)"
      >
        <X :size="14" />{{ t("Stop selected processes") }}
      </button>
    </div>
  </div>
  <div class="table-wrap">
    <table class="process-table explained-processes">
      <colgroup>
        <col class="process-selection-col" />
        <col class="process-id-col" />
        <col class="process-purpose-col" />
        <col />
        <col />
        <col />
        <col />
      </colgroup>
      <thead>
        <tr>
          <th class="process-selection-cell">
            <button
              role="checkbox"
              :aria-label="t('Select all visible processes')"
              :aria-checked="
                allSelected ? true : selectedProcesses.length ? 'mixed' : false
              "
              :class="['checkbox', { selected: selectedProcesses.length > 0 }]"
              @click="toggleAll"
            >
              <Check v-if="allSelected" :size="12" /><Minus
                v-else-if="selectedProcesses.length"
                :size="12"
              />
            </button>
          </th>
          <th>{{ t("Process") }}</th>
          <th>{{ t("Purpose") }}</th>
          <th>
            <abbr :title="t('RSS may include shared memory')">{{
              t("Memory")
            }}</abbr>
          </th>
          <th><abbr :title="t('100% CPU = one core')">CPU</abbr></th>
          <th>{{ t("State") }}</th>
          <th />
        </tr>
      </thead>
      <tbody>
        <template v-for="p in processes" :key="`${p.pid}:${p.startTime}`">
          <tr
            :class="{
              expanded: expanded === p.pid,
              'process-selected': selected.has(processKey(p)),
            }"
          >
            <td class="process-selection-cell">
              <button
                role="checkbox"
                :aria-label="t('Select {0} (PID {1})', p.name, p.pid)"
                :aria-checked="selected.has(processKey(p))"
                :class="['checkbox', { selected: selected.has(processKey(p)) }]"
                @click="toggle(p)"
              >
                <Check v-if="selected.has(processKey(p))" :size="12" />
              </button>
            </td>
            <td>
              <button
                class="process-expand"
                :aria-label="t('Inspect {0}', p.name)"
                :aria-expanded="expanded === p.pid"
                @click="expanded = expanded === p.pid ? null : p.pid"
              >
                <component
                  :is="expanded === p.pid ? ChevronDown : ChevronRight"
                  :size="13"
                /><span class="process-symbol"><Terminal :size="15" /></span
                ><span
                  ><strong>{{ p.name }}</strong
                  ><small
                    ><AgentIcon :id="p.agent" small />{{
                      AGENT_META[p.agent]?.name
                    }}
                    · {{ p.pid }}</small
                  ></span
                >
              </button>
            </td>
            <td class="process-purpose-cell">
              <strong>{{ purposeLabel(p.purpose) }}</strong>
              <small class="process-analysis-reason">{{
                assessmentReason(p)
              }}</small>
            </td>
            <td class="tabular">{{ bytes(p.memory) }}</td>
            <td class="tabular">{{ p.cpu.toFixed(1) }}%</td>
            <td>
              <Badge
                dot
                :tone="p.recommendation === 'review' ? 'amber' : 'neutral'"
                >{{
                  assessmentCategory(p) === "suggested"
                    ? t("Suggested cleanup")
                    : assessmentCategory(p) === "active"
                      ? t("Still working")
                      : assessmentCategory(p) === "protected"
                        ? t("Excluded")
                        : p.orphan
                          ? t("Parent exited")
                          : p.recommendation === "review"
                            ? t("Needs attention")
                            : t("Running")
                }}</Badge
              >
            </td>
            <td>
              <button
                class="button small danger-ghost"
                @click="$emit('stop', p)"
              >
                {{ t("Stop") }}<X :size="12" />
              </button>
            </td>
          </tr>
          <tr v-if="expanded === p.pid" class="process-detail-row">
            <td colspan="7">
              <div class="process-detail-content">
                <div class="process-explanation">
                  <h3>{{ purposeLabel(p.purpose) }}</h3>
                  <p>{{ t(p.purposeEvidence) }}</p>
                  <p :class="['process-recommendation', p.recommendation]">
                    {{ t(p.recommendationReason) }}
                  </p>
                  <p class="process-impact">{{ stopImpact(p.purpose) }}</p>
                </div>
                <dl class="process-metadata">
                  <div>
                    <dt>{{ t("Parent process") }}</dt>
                    <dd>
                      {{ parent(p)?.name || t("Not in observed tree") }} · PPID
                      {{ p.parentPid ?? "—" }}
                    </dd>
                  </div>
                  <div>
                    <dt>{{ t("Project / working directory") }}</dt>
                    <dd :title="p.workingDirectory">
                      {{
                        p.workingDirectory
                          ? compactPath(p.workingDirectory)
                          : t("Not available")
                      }}
                    </dd>
                  </div>
                  <div>
                    <dt>{{ t("Executable") }}</dt>
                    <dd :title="p.executable">
                      {{ compactPath(p.executable) || "—" }}
                    </dd>
                  </div>
                  <div>
                    <dt>{{ t("Observed ownership") }}</dt>
                    <dd>{{ t(p.evidence) }}</dd>
                  </div>
                  <div>
                    <dt>{{ t("Running for") }}</dt>
                    <dd>
                      {{ t("{0} minutes", Math.floor(p.runtimeSeconds / 60)) }}
                      · RAM {{ delta(p.memoryDelta) }} · CPU
                      {{ p.cpuDelta >= 0 ? "+" : "−"
                      }}{{ Math.abs(p.cpuDelta).toFixed(1) }} {{ t("pp") }}
                    </dd>
                  </div>
                </dl>
              </div>
            </td>
          </tr>
        </template>
      </tbody>
    </table>
  </div>
</template>
