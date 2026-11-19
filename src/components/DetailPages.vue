<script setup lang="ts">
import { t, categoryLabel } from "../lib/i18n";
import AgentRegistry from "./AgentRegistry.vue";
import UiTabs from "./UiTabs.vue";
import ProcessAnalytics from "./ProcessAnalytics.vue";
import { assessmentCategory, assessmentLabel, processCategories } from "../lib/processes";
import AgentStorageBreakdown from "./AgentStorageBreakdown.vue";
import UiSelect from "./UiSelect.vue";
import ContextHelp from "./ContextHelp.vue";
import CleanupRules from "./CleanupRules.vue";
import CleanupDiagnostics from "./CleanupDiagnostics.vue";
import ThemeSettings from "./ThemeSettings.vue";
import MonitoredDirectories from "./MonitoredDirectories.vue";
import ActivityView from "./ActivityView.vue";
import ProcessList from "./ProcessList.vue";
import { computed, ref } from "vue";
import {
  Plus,
  ScanSearch,
  ArrowRight,
  ArrowUpRight,
  CheckCheck,
  Cpu,
  Layers3,
  MemoryStick,
  Sparkles,
  Terminal,
  TriangleAlert,
} from "lucide-vue-next";
import type { FileEntry, Page, Process, Settings, Snapshot } from "../lib/types";
import { AGENT_META, bytes, relativeTime } from "../lib/types";
import AgentIcon from "./AgentIcon.vue";
import AgentTable from "./AgentTable.vue";
import Badge from "./Badge.vue";
import EmptyState from "./EmptyState.vue";
import FileRow from "./FileRow.vue";
import PageHeading from "./PageHeading.vue";
const props = defineProps<{
  page: Page;
  data: Snapshot;
  query: string;
  selected: Set<string>;
  preview: boolean;
  processesRefreshing: boolean;
}>();
const emit = defineEmits<{
  agent: [id: string];
  reveal: [path: string];
  add: [];
  adopt: [];
  navigate: [page: Page];
  stop: [process: Process];
  stopMany: [processes: Process[]];
  stopSuggested: [processes: Process[]];
  review: [];
  selected: [ids: Set<string>];
  settings: [settings: Settings];
  remove: [id: string];
  refresh: [];
  refreshProcesses: [];
  fullRescan: [];
}>();
const rulesOpen = ref(false);
const category = ref("all"),
  agent = ref("all"),
  sort = ref("size"),
  sessionSort = ref("recent"),
  processFilter = ref("all");
const processCategoryOptions = computed(() => [
  { value: "all", label: t("All processes"), count: props.data.processes.length },
  ...processCategories.map((category) => ({
    value: category,
    label: assessmentLabel(category),
    count: props.data.processes.filter((p) => assessmentCategory(p) === category).length,
  })),
]);
const search = (s: string) => s.toLowerCase().includes(props.query.toLowerCase());
const agents = computed(() => props.data.agents.filter((a) => search(`${a.name} ${a.id}`)));
const total = computed(() => props.data.agents.reduce((s, a) => s + a.storage, 0));
const files = computed(() =>
  props.data.files
    .filter(
      (f) =>
        (category.value === "all" || f.category === category.value) &&
        (agent.value === "all" || f.agent === agent.value) &&
        search(`${f.path} ${f.agent} ${f.category}`),
    )
    .sort((a, b) =>
      sort.value === "size"
        ? b.size - a.size
        : sort.value === "recent"
          ? b.modified - a.modified
          : a.relativePath.localeCompare(b.relativePath),
    ),
);
const sessions = computed(() =>
  props.data.sessions
    .filter(
      (s) =>
        (agent.value === "all" || s.agent === agent.value) &&
        search(`${s.id} ${s.project} ${s.agent}`),
    )
    .sort((a, b) => (sessionSort.value === "recent" ? b.modified - a.modified : b.size - a.size)),
);
const procs = computed(() =>
  props.data.processes.filter(
    (p) =>
      (processFilter.value === "all" || assessmentCategory(p) === processFilter.value) &&
      search(`${p.name} ${p.agent} ${AGENT_META[p.agent]?.name || ""} ${p.pid}`),
  ),
);
const candidates = computed(() =>
  props.data.files.filter(
    (f) =>
      f.eligible &&
      (category.value === "all" || f.category === category.value) &&
      search(`${f.path} ${f.agent}`),
  ),
);
const selectedFiles = computed(() =>
  props.data.files.filter((f) => f.eligible && props.selected.has(f.id)),
);
const selectedBytes = computed(() => selectedFiles.value.reduce((s, f) => s + f.size, 0));
const cleanupEmptyReason = computed(() => {
  if (props.query || category.value !== "all") return t("No cleanup candidates match this filter.");
  const blocked = props.data.cleanupSummary.find((b) => b.code === "active");
  if (blocked?.count)
    return t(
      "{0} older files ({1}) are retained while their Agent is running. They will be reassessed automatically when it exits.",
      blocked.count.toLocaleString(),
      bytes(blocked.bytes),
    );
  const pending = props.data.cleanupSummary.find((b) => b.code === "stability");
  if (pending?.count)
    return t(
      "{0} files are awaiting a second stable observation. AgentLens will recheck them automatically.",
      pending.count,
    );
  return t(
    "Files are within retention or protected by your rules. No files currently qualify for cleanup.",
  );
});

function toggle(f: FileEntry) {
  const n = new Set(props.selected);
  if (n.has(f.id)) n.delete(f.id);
  else n.add(f.id);
  emit("selected", n);
}
function selectAll() {
  const all = candidates.value.every((f) => props.selected.has(f.id));
  const n = new Set(props.selected);
  candidates.value.forEach((f) => (all ? n.delete(f.id) : n.add(f.id)));
  emit("selected", n);
}
function update(key: keyof Settings, value: Settings[keyof Settings]) {
  emit("settings", { ...props.data.settings, [key]: value });
}
const sum = (key: "memory" | "cpu" | "reclaimable") =>
  props.data.agents.reduce((s, a) => s + a[key], 0);
</script>
<template>
  <template v-if="page === 'Agents'">
    <PageHeading :title="t('Agents')">
      <button
        class="button primary"
        @click="emit('adopt')"
      >
        <Plus :size="16" />
        {{ t("Adopt Agent") }}
      </button>
    </PageHeading>
    <AgentRegistry :data="data" />
    <section class="panel">
      <div class="section-heading">
        <div>
          <h2>{{ t("Agent directory") }}</h2>
        </div>
      </div>
      <AgentTable
        :agents="agents.filter((a) => a.installed)"
        @select="emit('agent', $event)"
      />
      <EmptyState
        v-if="!agents.some((a) => a.installed)"
        :title="t('No matching agents')"
        :description="t('Try a different search or add a project directory.')"
      />
    </section>
    <section
      v-if="agents.some((a) => !a.installed && a.id !== 'unknown' && !a.id.startsWith('custom-'))"
      class="panel supported-panel"
    >
      <div class="section-heading">
        <div>
          <h2>{{ t("Not detected") }}</h2>
        </div>
      </div>
      <div class="supported-agents">
        <div
          v-for="a in agents.filter(
            (a) => !a.installed && a.id !== 'unknown' && !a.id.startsWith('custom-'),
          )"
          :key="a.id"
        >
          <AgentIcon :id="a.id" />
          <span>
            <strong>{{ t(a.name) }}</strong>
          </span>
        </div>
      </div>
    </section>
  </template>
  <template v-else-if="page === 'Storage'">
    <PageHeading :title="t('Storage')">
      <button
        class="button primary"
        @click="emit('navigate', 'Cleanup')"
      >
        <ScanSearch :size="16" />
        {{ t("Review cleanup") }}
      </button>
      <button
        class="button secondary"
        @click="emit('navigate', 'Settings')"
      >
        {{ t("Manage directories") }}
      </button>
    </PageHeading>
    <div class="storage-summary panel">
      <div>
        <span class="small-label">{{ t("TOTAL AGENT FOOTPRINT") }}</span>
        <h2>{{ bytes(total) }}</h2>
        <p>
          {{ data.fileCount.toLocaleString() }} {{ t("files across") }} {{ data.roots.length }}
          {{ t("monitored directories") }}
        </p>
      </div>
      <AgentStorageBreakdown
        :agents="data.agents"
        @select="emit('agent', $event)"
      />
    </div>
    <CleanupDiagnostics
      :data="data"
      @rules="rulesOpen = !rulesOpen"
      @refresh="emit('refresh')"
    />
    <CleanupRules
      v-if="rulesOpen"
      :settings="data.settings"
      @update="emit('settings', $event)"
    />
    <div class="page-toolbar">
      <UiTabs
        v-model="category"
        class="filter-tabs"
        :label="t('File categories')"
        :options="
          [
            'all',
            'sessions',
            'cache',
            'session_cache',
            'temporary',
            'logs',
            'settings',
            'project',
            'other',
          ].map((c) => ({
            value: c,
            label: c === 'all' ? t('All files') : categoryLabel(c),
          }))
        "
      />
    </div>
    <section class="panel">
      <div class="section-heading">
        <div>
          <h2>{{ t("Observed files") }}</h2>
          <p>
            {{ t("Showing") }} {{ Math.min(files.length, 100) }} {{ t("of") }}
            {{ data.fileCount.toLocaleString() }}
            {{ t("files") }}
          </p>
        </div>
        <div class="inline-controls">
          <UiSelect
            v-model="agent"
            :label="t('Filter by agent')"
            :options="[
              { value: 'all', label: t('All agents') },
              ...data.agents
                .filter((a) => a.installed)
                .map((a) => ({ value: a.id, label: t(a.name) })),
            ]"
          />
          <UiSelect
            v-model="sort"
            :label="t('Sort files')"
            :options="[
              { value: 'size', label: t('Largest first') },
              { value: 'recent', label: t('Recently modified') },
              { value: 'name', label: t('Name A–Z') },
            ]"
          />
        </div>
      </div>
      <div
        v-if="files.length"
        class="file-list"
      >
        <FileRow
          v-for="f in files.slice(0, 100)"
          :key="f.id"
          :file="f"
          @reveal="emit('reveal', f.path)"
        />
      </div>
      <EmptyState
        v-else
        :title="t('No files here')"
        :description="
          t('Try another category or search. Project source files are protected by default.')
        "
      />
    </section>
  </template>
  <template v-else-if="page === 'Sessions'">
    <PageHeading :title="t('Sessions')">
      <ContextHelp :label="t('Session data')">
        <p>
          {{
            t(
              "Only session ID and working-directory metadata are retained. Prompts and conversations stay private.",
            )
          }}
        </p>
      </ContextHelp>
    </PageHeading>
    <div class="page-toolbar">
      <UiTabs
        v-model="agent"
        class="filter-tabs"
        :label="t('Session Agents')"
        :options="
          ['all', ...data.agents.filter((a) => a.installed).map((a) => a.id)].map((a) => ({
            value: a,
            label: a === 'all' ? t('All sessions') : AGENT_META[a]?.name || a,
          }))
        "
      />
      <UiSelect
        v-model="sessionSort"
        :label="t('Sort sessions')"
        :options="[
          { value: 'recent', label: t('Most recent') },
          { value: 'size', label: t('Largest first') },
        ]"
      />
    </div>
    <section class="panel">
      <div class="section-heading">
        <div>
          <h2>{{ t("Observed sessions") }}</h2>
          <p>
            {{ sessions.length }}
            {{ t("sessions") }}
          </p>
        </div>
      </div>
      <div
        v-if="sessions.length"
        class="table-wrap"
      >
        <table class="sessions-table">
          <thead>
            <tr>
              <th>{{ t("Session & project") }}</th>
              <th>{{ t("Agent") }}</th>
              <th>{{ t("Attribution") }}</th>
              <th>{{ t("Size") }}</th>
              <th>{{ t("Modified") }}</th>
              <th />
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="s in sessions"
              :key="s.path"
            >
              <td>
                <div class="session-name">
                  <span class="session-symbol"><Terminal :size="16" /></span>
                  <div>
                    <strong>
                      {{
                        s.project === "Unknown project"
                          ? t(s.project)
                          : s.project.split(/[\\/]/).pop()
                      }}
                    </strong>
                    <small :title="s.id">
                      {{ s.id.slice(0, 25) }}{{ s.id.length > 25 ? "…" : "" }}
                    </small>
                  </div>
                </div>
              </td>
              <td>
                <div class="inline-agent">
                  <AgentIcon
                    :id="s.agent"
                    small
                  />
                  {{ AGENT_META[s.agent]?.name }}
                </div>
              </td>
              <td>
                <Badge :tone="s.confidence === 'metadata' ? 'green' : 'neutral'">
                  {{ s.confidence === "metadata" ? t("Session metadata") : t("Directory rule") }}
                </Badge>
              </td>
              <td class="tabular">{{ bytes(s.size) }}</td>
              <td class="secondary">{{ relativeTime(s.modified) }}</td>
              <td>
                <button
                  class="icon-button"
                  :aria-label="t('Reveal session {0}', s.id)"
                  @click="emit('reveal', s.path)"
                >
                  <ArrowUpRight :size="16" />
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <EmptyState
        v-else
        :title="t('No sessions found')"
        :description="
          t(
            'Sessions are indexed from bounded JSONL metadata. This list shows the 300 most recent sessions.',
          )
        "
      >
        <template #icon><Terminal :size="24" /></template>
      </EmptyState>
    </section>
  </template>
  <template v-else-if="page === 'Processes'">
    <PageHeading :title="t('Processes')">
      <ContextHelp :label="t('Resource measurement')">
        <p>{{ t("RSS may include shared memory") }}. {{ t("100% CPU = one core") }}.</p>
      </ContextHelp>
    </PageHeading>
    <div class="resource-strip">
      <div>
        <span
          class="resource-icon"
          :title="t('RSS may include shared memory')"
        >
          <MemoryStick :size="20" />
        </span>
        <span>
          {{ t("Agent memory") }}
          <strong>{{ bytes(sum("memory")) }}</strong>
        </span>
      </div>
      <div>
        <span
          class="resource-icon"
          :title="t('100% CPU = one core')"
        >
          <Cpu :size="20" />
        </span>
        <span>
          {{ t("Combined CPU") }}
          <strong>
            {{ sum("cpu").toFixed(1) }}
            <small>%</small>
          </strong>
        </span>
      </div>
      <div>
        <span class="resource-icon"><Layers3 :size="20" /></span>
        <span>
          {{ t("Running processes") }}
          <strong>{{ data.processes.length }}</strong>
        </span>
      </div>
      <div>
        <span class="resource-icon amber"><TriangleAlert :size="20" /></span>
        <span>
          {{ t("Needs attention") }}
          <strong>
            {{
              data.processes.filter((p) => ["suggested", "review"].includes(assessmentCategory(p)))
                .length
            }}
          </strong>
        </span>
      </div>
    </div>
    <ProcessAnalytics
      :data="data"
      :query="query"
    />
    <div class="page-toolbar">
      <UiTabs
        v-model="processFilter"
        class="filter-tabs"
        :label="t('Process categories')"
        :options="processCategoryOptions"
      />
    </div>
    <section class="panel process-results">
      <ProcessList
        :processes="procs"
        :all="data.processes"
        :monitoring="data.monitoring"
        :refreshing="processesRefreshing"
        :filter="processFilter"
        @refresh="emit('refreshProcesses')"
        @stop-suggested="emit('stopSuggested', $event)"
        @stop="emit('stop', $event)"
        @stop-many="emit('stopMany', $event)"
      />
    </section>
  </template>
  <template v-else-if="page === 'Cleanup'">
    <PageHeading :title="t('Cleanup')">
      <button
        class="button secondary"
        @click="rulesOpen = !rulesOpen"
      >
        {{ t("Adjust cleanup rules") }}
      </button>
      <button
        class="button primary"
        :disabled="data.scanning || !data.monitoring"
        @click="emit('refresh')"
      >
        <ScanSearch :size="16" />
        {{ t("Recheck files") }}
      </button>
    </PageHeading>
    <CleanupDiagnostics
      :data="data"
      :controls="false"
      @rules="rulesOpen = !rulesOpen"
      @refresh="emit('refresh')"
    />
    <CleanupRules
      v-if="rulesOpen"
      :settings="data.settings"
      @update="emit('settings', $event)"
    />
    <div class="page-toolbar">
      <UiTabs
        v-model="category"
        class="filter-tabs"
        :label="t('Cleanup categories')"
        :options="
          ['all', 'cache', 'temporary', 'logs', 'sessions'].map((c) => ({
            value: c,
            label: c === 'all' ? t('All candidates') : categoryLabel(c),
          }))
        "
      />
      <button
        class="text-button"
        @click="selectAll"
      >
        <CheckCheck :size="15" />
        {{
          candidates.length && candidates.every((f) => selected.has(f.id))
            ? t("Deselect all")
            : t("Select all")
        }}
      </button>
    </div>
    <section class="panel">
      <div class="section-heading">
        <div>
          <h2>{{ t("Cleanup candidates") }}</h2>
          <p>
            {{ candidates.length }}
            {{ t("files") }}
          </p>
        </div>
      </div>
      <div
        v-if="candidates.length"
        class="file-list"
      >
        <FileRow
          v-for="f in candidates"
          :key="f.id"
          :file="f"
          :checked="selected.has(f.id)"
          selectable
          @check="toggle(f)"
          @reveal="emit('reveal', f.path)"
        />
      </div>
      <EmptyState
        v-else
        :title="t('Nothing to clean up right now')"
        :description="cleanupEmptyReason"
      >
        <template #icon><Sparkles :size="25" /></template>
      </EmptyState>
      <div class="cleanup-action-bar">
        <div>
          <strong>{{ selectedFiles.length }} {{ t("files selected") }}</strong>
          <span>{{ bytes(selectedBytes) }}</span>
        </div>
        <button
          class="button primary"
          :disabled="!selectedFiles.length"
          @click="emit('review')"
        >
          {{ t("Review selected files") }}
          <ArrowRight :size="15" />
        </button>
      </div>
    </section>
  </template>
  <template v-else-if="page === 'Activity'">
    <ActivityView
      :events="data.activity"
      :history="data.history"
      :data="data"
      @navigate="emit('navigate', $event)"
      :query="query"
      :timestamp="data.timestamp"
    />
  </template>
  <template v-else-if="page === 'Settings'">
    <PageHeading :title="t('Settings')" />
    <div class="settings-layout">
      <section class="panel settings-panel">
        <div class="section-heading">
          <h2>{{ t("Language & theme") }}</h2>
        </div>
        <div class="setting-row">
          <div>
            <strong>{{ t("Display language") }}</strong>
          </div>
          <UiSelect
            :model-value="data.settings.language"
            :label="t('Display language')"
            :options="[
              { value: 'zh-CN', label: '简体中文' },
              { value: 'en-US', label: 'English' },
            ]"
            @update:model-value="update('language', $event)"
          />
        </div>
        <ThemeSettings
          :settings="data.settings"
          @update="emit('settings', $event)"
        />
      </section>
      <section class="panel settings-panel">
        <div class="section-heading">
          <h2>{{ t("Monitoring") }}</h2>
        </div>
        <div class="setting-row">
          <div>
            <strong>{{ t("Resource sampling") }}</strong>
          </div>
          <UiSelect
            :model-value="data.settings.interval"
            :label="t('Resource sampling interval')"
            :options="
              [2, 5, 10, 30].map((v) => ({
                value: v,
                label: t('Every {0} seconds', v),
              }))
            "
            @update:model-value="update('interval', $event)"
          />
        </div>
        <div class="setting-row">
          <div>
            <strong>{{ t("Keep resource history") }}</strong>
          </div>
          <UiSelect
            :model-value="data.settings.retentionDays"
            :label="t('History retention')"
            :options="[1, 7, 30].map((v) => ({ value: v, label: t('{0} days', v) }))"
            @update:model-value="update('retentionDays', $event)"
          />
        </div>
        <div class="setting-row">
          <div>
            <strong>{{ t("Memory in the menu bar") }}</strong>
          </div>
          <button
            role="switch"
            :aria-checked="data.settings.menuBar"
            :aria-label="t('Show memory in menu bar')"
            :class="['toggle', { checked: data.settings.menuBar }]"
            @click="update('menuBar', !data.settings.menuBar)"
          >
            <span />
          </button>
        </div>
      </section>
      <CleanupRules
        :settings="data.settings"
        @update="emit('settings', $event)"
      />
      <MonitoredDirectories
        :roots="data.roots"
        :automatic="data.settings.autoDiscover"
        :scanning="data.scanning"
        :monitoring="data.monitoring"
        @automatic="update('autoDiscover', $event)"
        @scan="emit('fullRescan')"
        @add="emit('add')"
        @remove="emit('remove', $event)"
        @reveal="emit('reveal', $event)"
      />
    </div>
  </template>
</template>
