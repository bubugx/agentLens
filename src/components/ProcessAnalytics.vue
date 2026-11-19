<script setup lang="ts">
import { computed, ref, shallowRef, watch } from "vue";
import { ArrowLeft, ChevronRight, Folder, Terminal } from "lucide-vue-next";
import { t } from "../lib/i18n";
import {
  AGENT_META,
  bytes,
  delta,
  compactPath,
  type Snapshot,
  type Process,
} from "../lib/types";
import {
  projectGroups,
  projectFor,
  type ResourcePoint,
} from "../lib/resources";
import { resourceHistory } from "../lib/client";
import { message } from "../lib/notifications";
import UiTabs from "./UiTabs.vue";
import UiSelect from "./UiSelect.vue";
import ResourceCurve from "./ResourceCurve.vue";
import ListPagination from "./ListPagination.vue";
const evidenceLabels: Record<string, string> = {
  working_directory: "From process working directory",
  monitored_directory: "Matched monitored project directory",
  parent_process: "Inherited from observed parent process",
  previous_observation: "From an earlier observation of this process",
  unassigned: "Project evidence unavailable",
};
const props = defineProps<{
  data: Snapshot;
  query: string;
  focusProcess?: Process | null;
}>();
const project = ref<string | null>(null),
  process = shallowRef<Process | null>(null),
  sort = ref("memory"),
  seconds = ref(3600),
  page = ref(1),
  pageSize = ref(10),
  history = shallowRef<ResourcePoint[]>([]),
  loading = ref(false),
  error = ref("");
const groups = computed(() =>
  projectGroups(props.data.processes, props.data.roots),
);
watch(
  () => props.focusProcess,
  (value) => {
    if (value) {
      process.value = value;
      project.value =
        projectFor(value, props.data.processes, props.data.roots).path ||
        `unassigned:${value.agent}`;
    }
  },
  { immediate: true },
);
const selectedProject = computed(() =>
  groups.value.find((g) => g.id === project.value),
);
const processes = computed(() =>
  project.value ? selectedProject.value?.processes || [] : props.data.processes,
);
const scope = computed(() =>
  process.value ? "process" : project.value ? "project" : "all",
);
const scopeId = computed(() =>
  process.value
    ? `${process.value.pid}:${process.value.startTime}`
    : project.value || "all",
);
const rows = computed(() =>
  project.value
    ? processes.value
        .filter((p) =>
          `${p.name} ${p.pid} ${p.agent}`
            .toLowerCase()
            .includes(props.query.toLowerCase()),
        )
        .map((p) => ({
          id: `${p.pid}:${p.startTime}`,
          name: p.name,
          path: `PID ${p.pid} · ${AGENT_META[p.agent]?.name || p.agent}`,
          cpu: p.cpu,
          memory: p.memory,
          cpuDelta: p.cpuDelta,
          memoryDelta: p.memoryDelta,
          count: 1,
          process: p,
        }))
    : groups.value
        .filter((g) =>
          `${g.path} ${AGENT_META[g.agent]?.name}`
            .toLowerCase()
            .includes(props.query.toLowerCase()),
        )
        .map((g) => ({
          id: g.id,
          name: g.path.split(/[\\/]/).pop() || t("Unassigned project"),
          path:
            g.path ||
            `${AGENT_META[g.agent]?.name || g.agent} · ${t("Project evidence unavailable")}`,
          cpu: g.cpu,
          memory: g.memory,
          cpuDelta: g.cpuDelta,
          memoryDelta: g.memoryDelta,
          count: g.processes.length,
          process: null as Process | null,
        })),
);
const ordered = computed(() =>
  [...rows.value].sort((a, b) =>
    sort.value === "cpu" ? b.cpu - a.cpu : b.memory - a.memory,
  ),
);
const current = computed(() => {
  const ps = process.value
    ? props.data.processes.filter(
        (p) =>
          p.pid === process.value!.pid &&
          p.startTime === process.value!.startTime,
      )
    : processes.value;
  return {
    time: props.data.timestamp,
    memory: ps.reduce((s, p) => s + p.memory, 0),
    cpu: ps.reduce((s, p) => s + p.cpu, 0),
    processes: ps.length,
  };
});
const points = computed(() => {
  const points = history.value.filter((p) => p.time < current.value.time);
  return [...points, current.value];
});
let generation = 0;
async function load() {
  const gen = ++generation;
  loading.value = true;
  error.value = "";
  try {
    const result = await resourceHistory(
      scope.value,
      scopeId.value,
      seconds.value,
    );
    if (gen === generation) history.value = result;
  } catch (e) {
    if (gen === generation) {
      error.value = String(e);
      message(
        t(
          "Resource history is temporarily unavailable. Current readings remain visible.",
        ),
        "error",
        { action: load, actionLabel: t("Retry") },
      );
    }
  } finally {
    if (gen === generation) loading.value = false;
  }
}
watch(
  [scope, scopeId, seconds],
  () => {
    history.value = [];
    load();
  },
  { immediate: true },
);
watch(
  () => Math.floor(props.data.timestamp / 30),
  () => load(),
);
watch([project, sort, () => props.query, pageSize], () => (page.value = 1));
watch(
  () => ordered.value.length,
  (n) =>
    (page.value = Math.min(
      page.value,
      Math.max(1, Math.ceil(n / pageSize.value)),
    )),
);
function drill(row: (typeof rows.value)[number]) {
  if (row.process) process.value = row.process;
  else {
    project.value = row.id;
    process.value = null;
  }
}
function back() {
  if (process.value) process.value = null;
  else project.value = null;
}
</script>
<template>
  <section class="panel process-analytics">
    <div class="section-heading">
      <div>
        <h2>{{ t("CPU and memory analysis") }}</h2>
        <p>
          {{
            t(
              "Compare projects, then inspect an individual process and its resource history.",
            )
          }}
        </p>
      </div>
      <UiTabs
        v-model="seconds"
        compact
        :label="t('Resource history range')"
        :options="[
          { value: 3600, label: t('Last hour') },
          { value: 86400, label: t('Last 24 hours') },
        ]"
      />
    </div>
    <div class="resource-breadcrumb">
      <button v-if="project || process" class="button small" @click="back">
        <ArrowLeft :size="14" />{{ t("Back") }}</button
      ><button
        class="text-button"
        @click="
          project = null;
          process = null;
        "
      >
        {{ t("All Agent processes") }}</button
      ><template v-if="project"
        ><ChevronRight :size="13" /><span :title="selectedProject?.path">{{
          selectedProject?.path
            ? compactPath(selectedProject.path)
            : t("Unassigned project")
        }}</span></template
      ><template v-if="process"
        ><ChevronRight :size="13" /><strong
          >{{ process.name }} · PID {{ process.pid }}</strong
        ></template
      >
    </div>
    <div class="resource-curves">
      <ResourceCurve
        :points="points"
        metric="cpu"
        :loading="loading"
      /><ResourceCurve :points="points" metric="memory" :loading="loading" />
    </div>
    <div v-if="process" class="resource-process-context">
      <strong>{{ t("Project attribution") }}</strong><span>{{
        projectFor(process, data.processes, data.roots).path ||
        t("Unassigned project")
      }}</span
      ><small>{{
        t(
          evidenceLabels[
            projectFor(process, data.processes, data.roots).evidence
          ] || "Project evidence unavailable",
        )
      }}</small>
      <small v-if="!current.processes">{{
        t(
          "This process is no longer observed. The curve shows recorded samples; the current reading is zero.",
        )
      }}</small>
    </div>
    <template v-else
      ><div class="resource-ranking-heading">
        <h3>
          {{
            project
              ? t("Processes in this project")
              : t("Projects by resource usage")
          }}
        </h3>
        <UiSelect
          v-model="sort"
          :label="t('Sort resource usage')"
          :options="[
            { value: 'memory', label: t('Memory · highest first') },
            { value: 'cpu', label: t('CPU · highest first') },
          ]"
        />
      </div>
      <div class="resource-ranking">
        <div class="resource-ranking-header">
          <span>{{ project ? t("Process") : t("Project") }}</span><span
            >CPU</span
          ><span>{{ t("Memory") }}</span
          ><span>{{ t("Processes") }}</span>
        </div>
        <button
          v-for="row in ordered.slice((page - 1) * pageSize, page * pageSize)"
          :key="row.id"
          class="resource-ranking-row"
          @click="drill(row)"
        >
          <span class="resource-ranking-name"
            ><component :is="project ? Terminal : Folder" :size="17" /><span
              ><strong>{{ row.name }}</strong
              ><small :title="row.path">{{
                compactPath(row.path)
              }}</small></span
            ></span
          ><span
            ><strong>{{ row.cpu.toFixed(1) }}%</strong
            ><small
              >{{ row.cpuDelta >= 0 ? "+" : "" }}{{ row.cpuDelta.toFixed(1) }}
              {{ t("pp") }}</small
            ></span
          ><span
            ><strong>{{ bytes(row.memory) }}</strong
            ><small>{{ delta(row.memoryDelta) }}</small></span
          ><span>{{ row.count }}<ChevronRight :size="14" /></span>
        </button>
        <p v-if="!ordered.length" class="resource-ranking-empty">
          {{ t("No matching running processes") }}
        </p>
      </div>
      <ListPagination
        v-if="ordered.length > 10"
        v-model:page="page"
        v-model:page-size="pageSize"
        :total="ordered.length"
        :label="t('Resource ranking pagination')"
    /></template>
    <p class="resource-analytics-note">
      {{
        t(
          "History is sampled every 30 seconds and retained for 24 hours. Changes in the ranking compare the last two resource observations. Project attribution uses directory and parent-process evidence.",
        )
      }}
    </p>
  </section>
</template>
