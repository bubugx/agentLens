<script setup lang="ts">
import { computed, nextTick, ref, shallowRef, watch } from "vue";
import { t, locale } from "../lib/i18n";
import {
  AGENT_META,
  bytes,
  delta,
  type Activity,
  type HistoryPoint,
  type Snapshot,
  type Process,
} from "../lib/types";
import {
  activityAction,
  activityCount,
  activityKind,
  matchesActivity,
  type ActivityFilter,
} from "../lib/activity";
import {
  Activity as ActivityIcon,
  ArrowDownLeft,
  ArrowUpRight,
  ChevronDown,
  FolderSearch,
  Terminal,
  Trash2,
} from "lucide-vue-next";
import PageHeading from "./PageHeading.vue";
import EmptyState from "./EmptyState.vue";
import UiSelect from "./UiSelect.vue";
import UiTabs from "./UiTabs.vue";
import ListPagination from "./ListPagination.vue";
import StorageActivityTrend from "./StorageActivityTrend.vue";
import ProcessAnalytics from "./ProcessAnalytics.vue";
import { projectFor } from "../lib/resources";
const props = defineProps<{
  events: Activity[];
  history: HistoryPoint[];
  data: Snapshot;
  query: string;
  timestamp: number;
}>();
const emit = defineEmits<{
  navigate: [page: "Cleanup" | "Processes" | "Settings"];
}>();
const focusProcess = shallowRef<Process | null>(null);
async function inspectProcess(p: Process) {
  filter.value = "process";
  focusProcess.value = p;
  await nextTick();
  document
    .querySelector(".process-analytics")
    ?.scrollIntoView({ block: "start" });
}
const filter = ref<ActivityFilter>("all"),
  period = ref("24h"),
  expanded = ref<string | null>(null);
const start = computed(() =>
  period.value === "all"
    ? Math.min(
        props.timestamp,
        ...props.events.map((e) => e.time),
        ...props.history.map((p) => p.time),
      )
    : props.timestamp - (period.value === "1h" ? 3600 : 86400),
);
const filtered = (events: Activity[], timestamp: number) =>
  events
    .filter(
      (a) =>
        matchesActivity(a, filter.value, props.query) &&
        (period.value === "all" ||
          a.time >= timestamp - (period.value === "1h" ? 3600 : 86400)),
    )
    .sort((a, b) => b.time - a.time);
const rows = computed(() => filtered(props.events, props.timestamp));
const tabs = computed(() => [
  { value: "all", label: t("All activity") },
  { value: "storage", label: t("Storage changes") },
  { value: "cleanup", label: t("Cleanups") },
  { value: "process", label: t("Processes") },
  { value: "discovery", label: t("Project discovery") },
]);
const storageEvents = computed(() =>
  rows.value.filter((a) => activityKind(a) === "storage"),
);
const summary = computed(() => {
  const items = rows.value,
    count = (action: string) =>
      items
        .filter((a) => activityAction(a) === action)
        .reduce((s, a) => s + activityCount(a), 0);
  if (filter.value === "storage")
    return [
      {
        label: "Observed increase",
        value: "+" + bytes(items.reduce((s, a) => s + Math.max(0, a.bytes), 0)),
      },
      {
        label: "Observed decrease",
        value:
          "−" + bytes(items.reduce((s, a) => s + Math.max(0, -a.bytes), 0)),
      },
      {
        label: "Net storage change",
        value: delta(items.reduce((s, a) => s + a.bytes, 0)),
      },
      { label: "Storage observations", value: items.length },
    ];
  if (filter.value === "cleanup")
    return [
      { label: "Cleanup operations", value: items.length },
      {
        label: "Files moved to Trash",
        value: items.reduce((s, a) => s + activityCount(a), 0),
      },
      {
        label: "Moved file size",
        value: bytes(items.reduce((s, a) => s + Math.max(0, -a.bytes), 0)),
      },
      {
        label: "Skipped or failed files",
        value: items.reduce((s, a) => s + (a.context?.failed || 0), 0),
      },
    ];
  if (filter.value === "process")
    return [
      {
        label: "Processes observed",
        value: count("observed") + count("detected"),
      },
      { label: "Observed exits", value: count("exited") },
      { label: "Attention signals", value: count("attention") },
      { label: "Stop requests", value: count("stop_requested") },
    ];
  if (filter.value === "discovery")
    return [
      {
        label: "Projects indexed",
        value: items.reduce((s, a) => s + activityCount(a), 0),
      },
      { label: "Discovery batches", value: items.length },
      {
        label: "Agents involved",
        value: new Set(items.map((a) => a.agent).filter((a) => a !== "system"))
          .size,
      },
      {
        label: "Latest discovery",
        value: items[0] ? shortTime(items[0].time) : "—",
      },
    ];
  return ["storage", "cleanup", "process", "discovery"].map((kind, i) => ({
    label: [
      "Storage observations",
      "Cleanup operations",
      "Process observations",
      "Discovery batches",
    ][i],
    value: items.filter((a) => activityKind(a) === kind).length,
  }));
});
const shortTime = (value: number) =>
  new Date(value * 1000).toLocaleTimeString(locale.value, {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
const time = (value: number) =>
  new Date(value * 1000).toLocaleTimeString(locale.value, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
const categoryInfo = computed(
  () =>
    ({
      all: {
        title: "Recent observations",
        description:
          "Storage, cleanup, process and project events in one timeline.",
        empty: "No matching activity yet",
        hint: "Monitoring observations and completed actions will appear here.",
      },
      storage: {
        title: "Storage change records",
        description:
          "Directory size changes observed between completed inventories.",
        empty: "No storage changes in this range",
        hint: "Complete another inventory or select a wider time range to see changes.",
      },
      cleanup: {
        title: "Cleanup history",
        description:
          "Completed cleanup attempts, moved files and skipped items.",
        empty: "No cleanup operations in this range",
        hint: "Cleanup history is recorded only after you confirm an actual cleanup.",
      },
      process: {
        title: "Process monitoring records",
        description:
          "Detected processes, exits, attention signals and your stop requests.",
        empty: "No process observations in this range",
        hint: "Monitoring records Agent processes when first observed, when they exit, or when their status needs attention.",
      },
      discovery: {
        title: "Project discovery records",
        description:
          "Indexed project directories with their Agent and discovery source.",
        empty: "No projects discovered in this range",
        hint: "An inventory records newly discovered project directories. Previously indexed projects remain managed in Settings.",
      },
    })[filter.value],
);
const categoryIcon = computed(() =>
  filter.value === "cleanup"
    ? Trash2
    : filter.value === "process"
      ? Terminal
      : filter.value === "discovery"
        ? FolderSearch
        : ActivityIcon,
);
const agents = computed(() => {
  const groups = new Map<
    string,
    { agent: string; up: number; down: number; count: number }
  >();
  storageEvents.value.forEach((a) => {
    const g = groups.get(a.agent) || {
      agent: a.agent,
      up: 0,
      down: 0,
      count: 0,
    };
    g.up += Math.max(0, a.bytes);
    g.down += Math.max(0, -a.bytes);
    g.count++;
    groups.set(a.agent, g);
  });
  return [...groups.values()].sort((a, b) => b.up + b.down - a.up - a.down);
});
const maxAgent = computed(() =>
  Math.max(1, ...agents.value.map((a) => a.up + a.down)),
);
const ledgerEvents = shallowRef([...props.events]);
const ledgerTimestamp = ref(props.timestamp);
const page = ref(1),
  pageSize = ref(10);
const snapshotIds = computed(
  () => new Set(ledgerEvents.value.map((a) => a.id)),
);
const incoming = computed(
  () =>
    filtered(props.events, props.timestamp).filter(
      (a) => !snapshotIds.value.has(a.id),
    ).length,
);
function refreshLedger() {
  ledgerEvents.value = [...props.events];
  ledgerTimestamp.value = props.timestamp;
  page.value = 1;
  expanded.value = null;
}
watch([filter, period, () => props.query], refreshLedger);
watch(pageSize, () => {
  page.value = 1;
  expanded.value = null;
});
watch(page, () => (expanded.value = null));
const groups = computed(() => {
  const map = new Map<
    string,
    {
      id: string;
      time: number;
      items: Activity[];
      bytes: number;
      kind: string;
      agent: string;
    }
  >();
  filtered(ledgerEvents.value, ledgerTimestamp.value).forEach((a) => {
    const id = ["growth", "decrease"].includes(a.kind)
      ? `${a.agent}:${a.kind}:${Math.floor(a.time / 60)}`
      : a.id;
    const group = map.get(id) || {
      id,
      time: a.time,
      items: [],
      bytes: 0,
      kind: a.kind,
      agent: a.agent,
    };
    group.items.push(a);
    group.bytes += a.bytes;
    map.set(id, group);
  });
  return [...map.values()].sort((a, b) => b.time - a.time);
});
const maxRow = computed(() =>
  Math.max(1, ...groups.value.map((g) => Math.abs(g.bytes))),
);
const visibleGroups = computed(() =>
  groups.value.slice(
    (page.value - 1) * pageSize.value,
    page.value * pageSize.value,
  ),
);
watch(
  () => groups.value.length,
  (n) =>
    (page.value = Math.min(
      page.value,
      Math.max(1, Math.ceil(n / pageSize.value)),
    )),
);
function title(a: Activity) {
  const cleanup = a.title.match(/^(\d+) files moved to Trash$/);
  if (cleanup) return t("{0} files moved to Trash", cleanup[1]);
  const stopped = a.title.match(/^Termination requested for (.+)$/);
  if (stopped) return t("Termination requested for {0}", stopped[1]);
  return t(a.title);
}
const icon = (kind: string, amount: number) =>
  kind === "cleanup"
    ? Trash2
    : kind === "process"
      ? Terminal
      : kind === "discovery"
        ? FolderSearch
        : amount < 0
          ? ArrowDownLeft
          : ArrowUpRight;
</script>
<template>
  <PageHeading :title="t('Activity')" />
  <div class="page-toolbar">
    <UiTabs
      v-model="filter"
      class="filter-tabs"
      :label="t('Activity categories')"
      :options="tabs"
    /><UiSelect
      v-model="period"
      :label="t('Activity time range')"
      :options="[
        { value: '1h', label: t('Last hour') },
        { value: '24h', label: t('Last 24 hours') },
        { value: 'all', label: t('All retained activity') },
      ]"
    />
  </div>
  <div class="activity-summary">
    <div v-for="item in summary" :key="item.label">
      <span>{{ t(item.label) }}</span><strong>{{ item.value }}</strong>
    </div>
  </div>
  <ProcessAnalytics
    v-if="filter === 'process'"
    :data="data"
    :query="query"
    :focus-process="focusProcess"
  />
  <div v-if="filter === 'storage'" class="activity-storage-workspace">
    <StorageActivityTrend
      :history="query ? [] : history"
      :events="storageEvents"
      :start="start"
      :end="timestamp"
    />
    <aside v-if="agents.length" class="panel activity-contributors">
      <div class="section-heading">
        <h2>{{ t("Storage changes by Agent") }}</h2>
      </div>
      <div v-for="a in agents" :key="a.agent" class="contributor">
        <div>
          <i
            :style="{
              background: AGENT_META[a.agent]?.color || 'var(--accent)',
            }"
          /><strong>{{ AGENT_META[a.agent]?.name || "AgentLens" }}</strong
          ><span>{{ a.count }} {{ t("events") }}</span>
        </div>
        <div class="contributor-bar">
          <span
            :style="{
              width: `${((a.up + a.down) / maxAgent) * 100}%`,
              background: AGENT_META[a.agent]?.color || 'var(--accent)',
            }"
          />
        </div>
        <small
          >+{{ bytes(a.up) }} <span>−{{ bytes(a.down) }}</span></small
        >
      </div>
    </aside>
  </div>
  <section class="panel activity-ledger" :data-category="filter">
    <div class="section-heading">
      <div>
        <h2>{{ t(categoryInfo.title) }}</h2>
        <p>{{ t(categoryInfo.description) }}</p>
      </div>
      <div class="inline-controls">
        <button
          v-if="incoming"
          class="button secondary small"
          @click="refreshLedger"
        >
          {{ t("Load {0} new observations", incoming) }}</button><span
          class="count-pill"
          >{{ groups.length }} {{ t("groups") }}</span
        ><button
          v-if="
            filter === 'cleanup' ||
            filter === 'process' ||
            filter === 'discovery'
          "
          class="button small"
          @click="
            emit(
              'navigate',
              filter === 'cleanup'
                ? 'Cleanup'
                : filter === 'process'
                  ? 'Processes'
                  : 'Settings',
            )
          "
        >
          {{
            filter === "cleanup"
              ? t("Review cleanup")
              : filter === "process"
                ? t("Process management")
                : t("Manage directories")
          }}
        </button>
      </div>
    </div>
    <div v-for="group in visibleGroups" :key="group.id" class="ledger-group">
      <div
        :class="[
          'ledger-row',
          {
            'has-process-readings':
              group.kind === 'process' &&
              group.items[0].context?.processes?.length,
          },
        ]"
      >
        <time :title="new Date(group.time * 1000).toLocaleString(locale)"
          >{{ shortTime(group.time)
          }}<small>{{
            new Date(group.time * 1000).toLocaleDateString(locale, {
              month: "2-digit",
              day: "2-digit",
            })
          }}</small></time
        ><span :class="['activity-symbol', group.kind]"
          ><component :is="icon(group.kind, group.bytes)" :size="16"
        /></span>
        <div class="ledger-copy">
          <div>
            <strong>{{
              group.items.length > 1
                ? t("Storage changes · {0} observations", group.items.length)
                : title(group.items[0])
            }}</strong><span class="ledger-agent">{{
              AGENT_META[group.agent]?.name || "AgentLens"
            }}</span>
          </div>
          <section
            v-if="
              group.kind === 'process' &&
              group.items[0].context?.processes?.length
            "
            class="ledger-process-readings"
            :aria-label="t('Process readings')"
          >
            <div class="ledger-process-header">
              <span>{{ t("Process / project") }}</span
              ><span>CPU</span><span>{{ t("Resident memory") }}</span
              ><span />
            </div>
            <div
              v-for="p in group.items[0].context.processes"
              :key="`${p.pid}:${p.startTime}`"
              class="ledger-process-reading"
            >
              <span
                ><strong>{{ p.name }} · PID {{ p.pid }}</strong
                ><small
                  :title="
                    projectFor(p, data.processes, data.roots).path ||
                    p.workingDirectory
                  "
                  >{{
                    projectFor(p, data.processes, data.roots).path ||
                    t("Unassigned project")
                  }}</small
                ></span
              >
              <span
                ><strong>{{ p.cpu.toFixed(1) }}%</strong
                ><small
                  >{{ p.cpuDelta >= 0 ? "+" : "" }}{{ p.cpuDelta.toFixed(1) }}
                  {{ t("pp") }}</small
                ></span
              >
              <span
                ><strong>{{ bytes(p.memory) }}</strong
                ><small>{{ delta(p.memoryDelta) }}</small></span
              >
              <button class="text-button" @click="inspectProcess(p)">
                {{ t("View resource trend") }}<ArrowUpRight :size="12" />
              </button>
            </div>
            <p class="ledger-process-caption">
              {{
                t(
                  "Resource readings at the time of this observation · up to 12 processes",
                )
              }}
              · {{ activityCount(group.items[0]) }} {{ t("processes") }}
            </p>
          </section>
          <p v-else-if="group.kind === 'discovery'">
            {{
              t(
                "{0} projects added to monitoring",
                activityCount(group.items[0]),
              )
            }}
          </p>
          <p v-else-if="group.kind === 'process'" class="process-event-detail">
            {{ group.items[0].detail }}
          </p>
          <p v-else-if="group.kind === 'cleanup'">
            {{ t(group.items[0].detail)
            }}<span v-if="group.items[0].context?.failed">
              ·
              {{
                t("{0} files skipped or failed", group.items[0].context!.failed)
              }}</span
            >
          </p>
          <button
            v-if="
              group.items.length > 1 || group.items[0].context?.paths.length
            "
            class="text-button"
            :aria-expanded="expanded === group.id"
            @click="expanded = expanded === group.id ? null : group.id"
          >
            {{
              expanded === group.id
                ? t("Collapse samples")
                : group.kind === "discovery"
                  ? t("View project directories")
                  : group.kind === "process"
                    ? t("View process readings")
                    : t("View {0} samples", group.items.length)
            }}<ChevronDown :size="12" />
          </button>
        </div>
        <div
          v-if="
            !(
              group.kind === 'process' &&
              group.items[0].context?.processes?.length
            )
          "
          class="ledger-change"
        >
          <template
            v-if="
              group.kind === 'growth' ||
              group.kind === 'decrease' ||
              group.kind === 'cleanup'
            "
            ><strong :class="{ decrease: group.bytes < 0 }">{{
              delta(group.bytes)
            }}</strong
            ><span
              ><i
                :style="{
                  width: `${group.bytes ? Math.max(3, (Math.abs(group.bytes) / maxRow) * 100) : 0}%`,
                }" /></span></template
          ><template v-else
            ><strong>{{ activityCount(group.items[0]) }}</strong
            ><small>{{
              group.kind === "discovery" ? t("projects") : t("processes")
            }}</small></template
          >
        </div>
      </div>
      <div v-if="expanded === group.id" class="ledger-samples">
        <template v-if="group.items[0].context?.paths.length"
          ><div
            v-for="path in group.items[0].context.paths"
            :key="path"
            class="ledger-project-path"
          >
            {{ path }}
          </div></template
        ><template v-else
          ><div v-for="event in group.items" :key="event.id">
            <time>{{ time(event.time) }}</time><span>{{ title(event) }}</span
            ><b>{{ delta(event.bytes) }}</b>
          </div></template
        >
      </div>
    </div>
    <ListPagination
      v-if="groups.length"
      v-model:page="page"
      v-model:page-size="pageSize"
      :total="groups.length"
      :label="t('Activity pagination')"
    />
    <EmptyState
      v-else
      :title="t(categoryInfo.empty)"
      :description="
        query
          ? t('No records match your search in this category.')
          : t(categoryInfo.hint)
      "
      ><template #icon><component :is="categoryIcon" :size="24" /></template
    ></EmptyState>
  </section>
</template>
