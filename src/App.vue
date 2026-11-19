<script setup lang="ts">
import packageInfo from "../package.json";
import { purposeLabel, stopImpact } from "./lib/processes";
import { t, locale, theme, customBase, customAccent, setLocale, setTheme } from "./lib/i18n";
import UiSelect from "./components/UiSelect.vue";
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef } from "vue";
import {
  Activity,
  ArrowRight,
  Check,
  Cpu,
  Folder,
  FolderOpen,
  HardDrive,
  Info,
  Layers3,
  LayoutDashboard,
  LoaderCircle,
  MemoryStick,
  MoreHorizontal,
  PanelLeft,
  Pause,
  Play,
  Plus,
  RefreshCw,
  Search,
  Settings as SettingsIcon,
  ShieldCheck,
  Sparkles,
  Terminal,
  Trash2,
  TriangleAlert,
  X,
} from "lucide-vue-next";
import * as client from "./lib/client";
import { listen } from "@tauri-apps/api/event";
import { AGENT_META, bytes, compactPath, delta, syncAgentDefinitions } from "./lib/types";
import type { ActionResult, CleanupPlan, Page, Process, Settings, Snapshot } from "./lib/types";
import NotificationHost from "./components/NotificationHost.vue";
import { message } from "./lib/notifications";
import { growthSignals } from "./lib/types";
import Overview from "./components/Overview.vue";
import ScanStatus from "./components/ScanStatus.vue";
import TrayPopover from "./components/TrayPopover.vue";
import DetailPages from "./components/DetailPages.vue";
import AgentIcon from "./components/AgentIcon.vue";
import Badge from "./components/Badge.vue";
import Modal from "./components/Modal.vue";
import ContextHelp from "./components/ContextHelp.vue";
import FileRow from "./components/FileRow.vue";
import AdoptAgentDialog from "./components/AdoptAgentDialog.vue";
import ProcessBatchDialog from "./components/ProcessBatchDialog.vue";
const data = shallowRef<Snapshot | null>(null),
  error = ref(""),
  page = ref<Page>("Overview"),
  query = ref(""),
  searchInput = ref<HTMLInputElement>(),
  quicklookAnchor = ref<HTMLElement>(),
  sidebar = ref(true),
  selected = ref(new Set<string>()),
  busy = ref(false),
  menu = ref(false),
  refreshing = ref(false),
  processesRefreshing = ref(false),
  agentDetail = ref<string | null>(null),
  plan = ref<CleanupPlan | null>(null),
  stopping = ref<Process | null>(null),
  batchStopping = shallowRef<Process[] | null>(null),
  batchSuggested = ref(false),
  adoptOpen = ref(false),
  addOpen = ref(false),
  projectPath = ref(""),
  projectAgent = ref("claude"),
  result = ref<ActionResult | null>(null),
  about = ref(false),
  quicklook = ref(false);
let unlistenProgress: (() => void) | undefined;
let unlistenNavigate: (() => void) | undefined;
let unlisten: (() => void) | undefined, refreshTimer: ReturnType<typeof setTimeout> | undefined;
const preview = !client.native;
const navigation = [
  { page: "Overview" as Page, icon: LayoutDashboard },
  { page: "Agents" as Page, icon: Layers3 },
  { page: "Storage" as Page, icon: HardDrive },
  { page: "Sessions" as Page, icon: Terminal },
  { page: "Processes" as Page, icon: Cpu },
];
const utilities = [
  { page: "Cleanup" as Page, icon: Sparkles },
  { page: "Activity" as Page, icon: Activity },
];
const agent = computed(() => data.value?.agents.find((a) => a.id === agentDetail.value));
const agentFiles = computed(
  () => data.value?.files.filter((f) => f.agent === agentDetail.value).slice(0, 6) || [],
);
const reclaimable = computed(() => data.value?.agents.reduce((s, a) => s + a.reclaimable, 0) || 0);

function reload() {
  window.location.reload();
}
function notify(text: string) {
  message(t(text), "success");
}
function fail(e: unknown) {
  message(t(e instanceof Error ? e.message : String(e)), "error");
}
const warningKeys = new Set<string>();
function navigate(p: Page) {
  page.value = p;
  query.value = "";
  menu.value = false;
  quicklook.value = false;
  nextTick(() => document.querySelector(".main-scroll")?.scrollTo({ top: 0 }));
}
function receive(snapshot: Snapshot) {
  if (preview) {
    snapshot = {
      ...snapshot,
      settings: {
        ...snapshot.settings,
        language: locale.value,
        theme: theme.value,
        customBase: customBase.value,
        customAccent: customAccent.value,
      },
    };
  } else {
    setLocale(snapshot.settings.language || "zh-CN");
    setTheme(
      snapshot.settings.theme || "light",
      snapshot.settings.customBase,
      snapshot.settings.customAccent,
    );
  }
  syncAgentDefinitions(snapshot.agentDefinitions);
  data.value = snapshot;
  for (const warning of snapshot.warnings) {
    if (!warningKeys.has(warning)) {
      warningKeys.add(warning);
      message(t(warning), "warning", {
        action: () => (about.value = true),
        actionLabel: t("Details"),
        duration: 10000,
      });
    }
  }
  for (const signal of growthSignals(snapshot.history)) {
    const key = `growth:${signal.metric}:${Math.floor(snapshot.timestamp / 600)}`;
    if (!warningKeys.has(key)) {
      warningKeys.add(key);
      message(
        t(
          "{0} grew by {1} over {2} minutes",
          signal.metric === "storage" ? t("Storage") : t("Agent memory"),
          delta(signal.change),
          signal.minutes,
        ),
        "warning",
        {
          action: () => navigate(signal.metric === "storage" ? "Storage" : "Processes"),
          actionLabel: t("Review"),
        },
      );
    }
  }

  if (!snapshot.scanning) refreshing.value = false;
}
onMounted(async () => {
  if (client.native) document.documentElement.classList.add("native-host");
  try {
    unlisten = await client.subscribe(receive);
    unlistenProgress = await client.subscribeProgress((progress) => {
      if (data.value)
        data.value = {
          ...data.value,
          scanProgress: progress,
          scanning: !["complete", "paused", "error"].includes(progress.phase),
        };
    });
    if (client.native) {
      unlistenNavigate = await listen<string>("navigate", (e) => navigate(e.payload as Page));
    }
    receive(await client.snapshot());
  } catch (e) {
    error.value = String(e);
  }
  window.addEventListener("keydown", shortcut);
  document.addEventListener("pointerdown", dismissQuicklookOutside);
});
onBeforeUnmount(() => {
  unlisten?.();
  unlistenProgress?.();
  unlistenNavigate?.();
  window.removeEventListener("keydown", shortcut);
  document.removeEventListener("pointerdown", dismissQuicklookOutside);
  if (refreshTimer) clearTimeout(refreshTimer);
});
function shortcut(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key === "k") {
    e.preventDefault();
    searchInput.value?.focus();
  }
  if (e.key === "Escape") {
    menu.value = false;
    quicklook.value = false;
    query.value = "";
  }
  if ((e.metaKey || e.ctrlKey) && e.key === ",") {
    e.preventDefault();
    navigate("Settings");
  }
}
function dismissQuicklookOutside(event: PointerEvent) {
  if (!quicklookAnchor.value?.contains(event.target as Node)) quicklook.value = false;
}
async function refresh() {
  try {
    refreshing.value = true;
    await client.refresh();
    if (preview) notify("Sample workspace refreshed");
    refreshTimer = setTimeout(() => (refreshing.value = false), 12000);
  } catch (e) {
    refreshing.value = false;
    fail(e);
  }
}
async function refreshProcesses() {
  if (processesRefreshing.value) return;
  processesRefreshing.value = true;
  try {
    receive(await client.refreshProcesses());
    notify("Process analysis updated");
  } catch (error) {
    fail(error);
  } finally {
    processesRefreshing.value = false;
  }
}
async function fullRescan() {
  try {
    await client.fullRescan();
  } catch (e) {
    fail(e);
  }
}
async function toggleMonitoring() {
  try {
    await client.monitoring(!data.value?.monitoring);
    if (data.value && client.native)
      data.value = { ...data.value, monitoring: !data.value.monitoring };
    menu.value = false;
  } catch (e) {
    fail(e);
  }
}
async function settings(value: Settings) {
  setLocale(value.language);
  setTheme(value.theme, value.customBase, value.customAccent);
  try {
    await client.saveSettings(value);
    if (data.value && client.native) data.value = { ...data.value, settings: value };
    notify("Preferences saved");
  } catch (e) {
    fail(e);
  }
}
async function reveal(path: string) {
  try {
    await client.reveal(path);
    if (preview) notify("Sample path copied to clipboard");
  } catch (e) {
    fail(e);
  }
}
async function chooseDirectory() {
  if (!client.native) {
    projectPath.value = "/Users/alex/Developer/new-project";
    return;
  }
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const chosen = await open({
      directory: true,
      multiple: false,
      title: t("Choose a project to monitor"),
    });
    if (typeof chosen === "string") projectPath.value = chosen;
  } catch (e) {
    fail(e);
  }
}
function addProject() {
  projectPath.value = "";
  addOpen.value = true;
  menu.value = false;
}
async function add() {
  if (!projectPath.value) return;
  busy.value = true;
  try {
    await client.addRoot(projectPath.value, projectAgent.value);
    addOpen.value = false;
    notify(preview ? "Sample project added" : "Project added. The first scan will begin shortly.");
    if (client.native) await refresh();
  } catch (e) {
    fail(e);
  } finally {
    busy.value = false;
  }
}
async function remove(id: string) {
  try {
    await client.removeRoot(id);
    notify("Directory removed from monitoring. Files were kept.");
  } catch (e) {
    fail(e);
  }
}
async function review() {
  busy.value = true;
  try {
    plan.value = await client.prepareCleanup([...selected.value]);
  } catch (e) {
    fail(e);
  } finally {
    busy.value = false;
  }
}
async function cleanup() {
  if (!plan.value) return;
  busy.value = true;
  try {
    result.value = await client.executeCleanup(plan.value);
    plan.value = null;
    selected.value = new Set();
    notify(
      preview
        ? "Preview cleanup complete — no real files changed"
        : "Selected files moved to Trash",
    );
    if (client.native) await refresh();
  } catch (e) {
    fail(e);
    plan.value = null;
  } finally {
    busy.value = false;
  }
}
async function stop() {
  if (!stopping.value) return;
  busy.value = true;
  try {
    await client.stopProcess(stopping.value.pid, stopping.value.startTime);
    stopping.value = null;
    notify(
      preview
        ? "Sample process stopped"
        : "Termination requested. The next sample will confirm its state.",
    );
  } catch (e) {
    fail(e);
  } finally {
    busy.value = false;
  }
}
async function windowAction(action: "close" | "minimize" | "maximize") {
  if (!client.native) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const win = getCurrentWindow();
  if (action === "close") await win.close();
  else if (action === "minimize") await win.minimize();
  else await win.toggleMaximize();
}
function search() {
  if (query.value && ["Overview", "Settings"].includes(page.value)) page.value = "Storage";
}
</script>
<template>
  <div
    v-if="data"
    :class="[
      'app-shell',
      {
        'sidebar-hidden': !sidebar,
        'native-app': client.native,
      },
    ]"
  >
    <aside class="sidebar glass">
      <div
        class="window-controls"
        data-tauri-drag-region
      >
        <template v-if="!client.native || data.platform !== 'macos'">
          <button
            class="traffic close"
            :aria-label="t('Close window')"
            @click="windowAction('close')"
          />
          <button
            class="traffic minimize"
            :aria-label="t('Minimize window')"
            @click="windowAction('minimize')"
          />
          <button
            class="traffic maximize"
            :aria-label="t('Maximize window')"
            @click="windowAction('maximize')"
          />
        </template>
      </div>
      <button
        class="brand"
        :aria-label="t('About AgentLens')"
        @click="about = true"
      >
        <img
          src="/icon.svg"
          alt=""
        />
        <span>AgentLens</span>
      </button>
      <nav :aria-label="t('Main navigation')">
        <button
          v-for="item in navigation"
          :key="item.page"
          :class="['nav-item', { active: page === item.page }]"
          :aria-current="page === item.page ? 'page' : undefined"
          :aria-label="t(item.page)"
          @click="navigate(item.page)"
        >
          <component
            :is="item.icon"
            :size="17"
            :stroke-width="1.7"
          />
          <span>{{ t(item.page) }}</span>
          <span
            v-if="item.page === 'Agents'"
            class="nav-count"
          >
            {{ data.agents.filter((a) => a.installed).length }}
          </span>
          <i
            v-if="item.page === 'Processes' && data.processes.some((p) => p.orphan)"
            class="attention-dot"
          />
        </button>
      </nav>
      <nav :aria-label="t('Utilities')">
        <button
          v-for="item in utilities"
          :key="item.page"
          :class="['nav-item', { active: page === item.page }]"
          :aria-current="page === item.page ? 'page' : undefined"
          @click="navigate(item.page)"
        >
          <component
            :is="item.icon"
            :size="17"
            :stroke-width="1.7"
          />
          <span>{{ t(item.page) }}</span>
          <span
            v-if="item.page === 'Cleanup' && reclaimable"
            class="cleanup-count"
          >
            {{ bytes(reclaimable) }}
          </span>
        </button>
      </nav>
      <div class="sidebar-spacer" />
      <button
        :class="['nav-item settings-nav', { active: page === 'Settings' }]"
        :aria-label="t('Settings')"
        @click="navigate('Settings')"
      >
        <SettingsIcon
          :size="17"
          :stroke-width="1.7"
        />
        <span>{{ t("Settings") }}</span>
        <kbd>⌘ ,</kbd>
      </button>
    </aside>
    <div class="main-layout">
      <header
        class="titlebar"
        data-tauri-drag-region
      >
        <div class="titlebar-location">
          <button
            class="icon-button sidebar-toggle"
            :aria-label="t('Toggle sidebar')"
            @click="sidebar = !sidebar"
          >
            <PanelLeft :size="17" />
          </button>
          <Badge
            v-if="preview"
            tone="preview"
          >
            {{ t("Preview · sample data") }}
          </Badge>
        </div>
        <div class="titlebar-actions">
          <div
            ref="quicklookAnchor"
            class="quicklook-anchor"
          >
            <button
              class="quick-preview-button"
              :aria-label="t('Menu bar preview')"
              aria-haspopup="dialog"
              :aria-expanded="quicklook"
              @click="quicklook = !quicklook"
            >
              <MemoryStick :size="12" />
              {{ bytes(data.agents.reduce((s, a) => s + a.memory, 0)) }}
              <Cpu :size="12" />
              {{ data.agents.reduce((s, a) => s + a.cpu, 0).toFixed(0) }}%
            </button>
            <TrayPopover
              v-if="quicklook"
              embedded
              :data="data"
              @close="quicklook = false"
              @navigate="navigate($event as Page)"
            />
          </div>
          <label class="search-input">
            <Search :size="14" />
            <input
              ref="searchInput"
              v-model="query"
              :placeholder="t('Search files, agents…')"
              :aria-label="t('Search workspace')"
              @input="search"
            />
            <button
              v-if="query"
              class="search-clear"
              :aria-label="t('Clear search')"
              @click="query = ''"
            >
              <X :size="12" />
            </button>
            <kbd v-else>{{ t("⌘ K") }}</kbd>
          </label>
          <span class="toolbar-divider" />
          <button
            :class="['icon-button', { spinning: refreshing }]"
            :aria-label="t('Refresh workspace')"
            :disabled="refreshing || data.scanning"
            @click="refresh"
          >
            <RefreshCw :size="16" />
          </button>
          <div class="menu-anchor">
            <button
              class="icon-button"
              :aria-label="t('Workspace menu')"
              :aria-expanded="menu"
              @click="menu = !menu"
            >
              <MoreHorizontal :size="19" />
            </button>
            <div
              v-if="menu"
              class="dropdown-menu glass"
            >
              <button @click="addProject">
                <Plus :size="15" />
                {{ t("Add manually") }}
              </button>
              <button @click="toggleMonitoring">
                <Pause
                  v-if="data.monitoring"
                  :size="15"
                />
                <Play
                  v-else
                  :size="15"
                />
                {{ data.monitoring ? t("Pause monitoring") : t("Resume monitoring") }}
              </button>
              <button @click="navigate('Settings')">
                <SettingsIcon :size="15" />
                {{ t("Preferences") }}
                <kbd>⌘ ,</kbd>
              </button>
              <button
                @click="
                  about = true;
                  menu = false;
                "
              >
                <Info :size="15" />
                {{ t("About AgentLens") }}
              </button>
            </div>
          </div>
        </div>
      </header>
      <main
        class="main-scroll"
        @click="menu = false"
      >
        <div class="content">
          <ScanStatus
            :data="data"
            @toggle="toggleMonitoring"
            @rescan="fullRescan"
          />
          <Overview
            v-if="page === 'Overview'"
            :data="data"
            :preview="preview"
            @navigate="navigate"
            @agent="agentDetail = $event"
          />
          <DetailPages
            v-else
            :key="page"
            :processes-refreshing="processesRefreshing"
            :page="page"
            :data="data"
            :query="query"
            :selected="selected"
            :preview="preview"
            @navigate="navigate"
            @agent="agentDetail = $event"
            @reveal="reveal"
            @adopt="adoptOpen = true"
            @add="addProject"
            @stop="stopping = $event"
            @stop-many="
              batchSuggested = false;
              batchStopping = [...$event];
            "
            @stop-suggested="
              batchSuggested = true;
              batchStopping = [...$event];
            "
            @review="review"
            @refresh="refresh"
            @refresh-processes="refreshProcesses"
            @full-rescan="fullRescan"
            @selected="selected = $event"
            @settings="settings"
            @remove="remove"
          />
        </div>
      </main>
    </div>
    <NotificationHost />
    <Modal
      v-if="agentDetail && agent"
      :title="agent.name"
      wide
      @close="agentDetail = null"
    >
      <div class="agent-modal-summary">
        <AgentIcon :id="agent.id" />
        <Badge
          dot
          :tone="agent.processes ? 'green' : 'neutral'"
        >
          {{ agent.processes ? t("Active") : t("Idle") }}
        </Badge>
        <span class="agent-summary-count">
          {{ agent.sessions }} {{ t("sessions ·") }} {{ agent.files.toLocaleString() }}
          {{ t("files") }}
        </span>
        <ContextHelp :label="t('Storage association')">
          <p>
            {{
              t(
                "Known directory conventions or a user-assigned project label. These rules do not prove which process wrote each file.",
              )
            }}
          </p>
        </ContextHelp>
      </div>
      <div class="detail-metrics">
        <div>
          <HardDrive :size="16" />
          <span>{{ t("Storage") }}</span>
          <strong>{{ bytes(agent.storage) }}</strong>
        </div>
        <div>
          <MemoryStick :size="16" />
          <span>{{ t("Memory") }}</span>
          <strong>{{ bytes(agent.memory) }}</strong>
        </div>
        <div>
          <Cpu :size="16" />
          <span>CPU</span>
          <strong>{{ agent.cpu.toFixed(1) }}%</strong>
        </div>
      </div>
      <div class="mini-section-heading">
        <h3>{{ t("Largest observed files") }}</h3>
        <span>{{ delta(agent.growth) }} {{ t("since baseline") }}</span>
      </div>
      <div class="modal-file-list">
        <FileRow
          v-for="f in agentFiles"
          :key="f.id"
          :file="f"
          @reveal="reveal(f.path)"
        />
      </div>
      <template #footer>
        <button
          class="button secondary"
          @click="
            agentDetail = null;
            page = 'Sessions';
          "
        >
          {{ t("View sessions") }}
          <Terminal :size="14" />
        </button>
        <button
          class="button primary"
          @click="
            agentDetail = null;
            page = 'Storage';
          "
        >
          {{ t("Explore storage") }}
          <ArrowRight :size="14" />
        </button>
      </template>
    </Modal>
    <Modal
      v-if="addOpen"
      :title="t('Watch a project')"
      @close="!busy && (addOpen = false)"
    >
      <div class="form-field">
        <label for="project-agent">{{ t("Associated agent") }}</label>
        <UiSelect
          id="project-agent"
          v-model="projectAgent"
          :label="t('Associated agent')"
          :options="
            Object.entries(AGENT_META)
              .filter(([, a]) => a.enabled !== false)
              .map(([id, a]) => ({
                value: id,
                label: a.name,
              }))
          "
        />
      </div>
      <div class="form-field">
        <label for="project-path">{{ t("Project directory") }}</label>
        <div class="path-picker">
          <input
            id="project-path"
            v-model="projectPath"
            :readonly="client.native"
            :placeholder="t('Choose a directory…')"
          />
          <button
            class="button secondary"
            @click="chooseDirectory"
          >
            <FolderOpen :size="15" />
            {{ t("Browse") }}
          </button>
        </div>
      </div>
      <div class="notice">
        <ShieldCheck :size="18" />
        <span>
          {{
            t(
              "Project files are monitored and protected from cleanup. Agent association is your label; generated files may have other writers. Dependency and build directories are skipped.",
            )
          }}
        </span>
      </div>
      <template #footer>
        <button
          class="button secondary"
          :disabled="busy"
          @click="addOpen = false"
        >
          {{ t("Cancel") }}
        </button>
        <button
          class="button primary"
          :disabled="!projectPath || busy"
          @click="add"
        >
          <LoaderCircle
            v-if="busy"
            :size="15"
            class="spin"
          />
          <Plus
            v-else
            :size="15"
          />
          {{ preview ? t("Add sample project") : t("Start watching") }}
        </button>
      </template>
    </Modal>
    <Modal
      v-if="plan"
      :title="t('A final look before cleanup')"
      :subtitle="
        t(
          '{0} files · {1} logical size · plan expires after 60 seconds',
          plan.items.length,
          bytes(plan.total),
        )
      "
      wide
      @close="!busy && (plan = null)"
    >
      <div class="notice">
        <Trash2 :size="19" />
        <span>
          {{
            preview
              ? t("This is a preview. No real files will be moved.")
              : t(
                  "Selected files will move to system Trash. Restore them there if needed. Moving to Trash does not immediately release disk space.",
                )
          }}
        </span>
      </div>
      <div class="review-files">
        <div
          v-for="f in plan.items"
          :key="f.id"
          class="review-file"
        >
          <span class="file-kind"><Folder :size="17" /></span>
          <div>
            <strong>{{ f.relativePath }}</strong>
            <span>{{ compactPath(f.path) }}</span>
            <small>{{ t(f.reason) }}</small>
          </div>
          <b>{{ bytes(f.size) }}</b>
        </div>
      </div>
      <div class="review-total">
        <span>{{ t("Total selected") }}</span>
        <strong>{{ bytes(plan.total) }}</strong>
      </div>
      <template #footer>
        <button
          class="button secondary"
          :disabled="busy"
          @click="plan = null"
        >
          {{ t("Keep files") }}
        </button>
        <button
          class="button primary"
          :disabled="busy"
          @click="cleanup"
        >
          <LoaderCircle
            v-if="busy"
            :size="15"
            class="spin"
          />
          <Trash2
            v-else
            :size="15"
          />
          {{ preview ? t("Run preview cleanup") : t("Move to Trash") }}
        </button>
      </template>
    </Modal>
    <AdoptAgentDialog
      v-if="adoptOpen"
      @close="adoptOpen = false"
      @adopted="
        receive($event);
        adoptOpen = false;
      "
    />
    <ProcessBatchDialog
      v-if="batchStopping"
      :processes="batchStopping"
      :suggested="batchSuggested"
      :preview="preview"
      @close="batchStopping = null"
    />
    <Modal
      v-if="stopping"
      :title="t('Stop {0}?', stopping.name)"
      @close="!busy && (stopping = null)"
    >
      <div class="process-confirm">
        <Terminal :size="26" />
        <div>
          <strong>{{ stopping.name }}</strong>
          <span>PID {{ stopping.pid }} · {{ AGENT_META[stopping.agent]?.name }}</span>
        </div>
        <Badge :tone="stopping.orphan ? 'amber' : 'neutral'">
          {{ stopping.orphan ? t("Parent exited") : t("Running") }}
        </Badge>
      </div>
      <div class="stop-purpose">
        <strong>{{ purposeLabel(stopping.purpose) }}</strong>
        <p>{{ t(stopping.recommendationReason) }}</p>
        <p>{{ stopImpact(stopping.purpose) }}</p>
      </div>
      <dl class="process-facts">
        <div>
          <dt>{{ t("Memory") }}</dt>
          <dd>{{ bytes(stopping.memory) }}</dd>
        </div>
        <div>
          <dt>CPU</dt>
          <dd>{{ stopping.cpu.toFixed(1) }}%</dd>
        </div>
        <div>
          <dt>{{ t("Evidence") }}</dt>
          <dd>{{ t(stopping.evidence) }}</dd>
        </div>
      </dl>
      <div class="notice amber">
        <TriangleAlert :size="18" />
        <span>
          {{
            preview
              ? t("Preview only. No real process will be stopped.")
              : t(
                  "Unsaved work or a running task may be interrupted. macOS/Linux send SIGTERM; Windows requests process termination.",
                )
          }}
        </span>
      </div>
      <template #footer>
        <button
          class="button secondary"
          :disabled="busy"
          @click="stopping = null"
        >
          {{ t("Keep running") }}
        </button>
        <button
          class="button danger"
          :disabled="busy"
          @click="stop"
        >
          <LoaderCircle
            v-if="busy"
            :size="15"
            class="spin"
          />
          {{ preview ? t("Stop sample process") : t("Stop process") }}
        </button>
      </template>
    </Modal>
    <Modal
      v-if="result"
      :title="t('Cleanup results')"
      @close="result = null"
    >
      <div class="cleanup-result">
        <span><Check :size="30" /></span>
        <h2>{{ bytes(result.bytes) }}</h2>
        <p>
          {{ result.succeeded }}
          {{ preview ? t("sample files cleaned up") : t("files moved to Trash") }}
        </p>
      </div>
      <div
        v-if="result.failures.length"
        class="notice amber"
      >
        <TriangleAlert :size="18" />
        <div>
          <strong>{{ result.failures.length }} {{ t("files could not be moved") }}</strong>
          <p
            v-for="failure in result.failures"
            :key="failure"
          >
            {{ failure }}
          </p>
        </div>
      </div>
      <p class="result-help">
        {{
          preview
            ? t("No files on this computer were changed.")
            : t(
                "Files can be recovered from system Trash. Free disk space may remain unchanged until Trash is emptied.",
              )
        }}
      </p>
      <template #footer>
        <button
          class="button secondary"
          @click="
            result = null;
            navigate('Activity');
          "
        >
          {{ t("View activity") }}
        </button>
        <button
          class="button primary"
          @click="result = null"
        >
          {{ t("All done") }}
          <Check :size="14" />
        </button>
      </template>
    </Modal>
    <Modal
      v-if="about"
      :title="t('A little more about AgentLens')"
      :subtitle="t('Your agents, in focus. Version {0}', packageInfo.version)"
      @close="about = false"
    >
      <div class="about-logo">
        <img
          src="/icon.svg"
          alt="AgentLens"
        />
        <div>
          <strong>{{ t("Built to keep your workspace light.") }}</strong>
          <p>{{ t("Tauri 2 + Vue 3 + TypeScript + Rust") }}</p>
        </div>
      </div>
      <p class="about-copy">
        {{
          t(
            "AgentLens observes known agent directories and same-user process trees. File sizes are logical sizes. Session discovery is bounded and never retains conversation content. Unobserved writers and process-to-session mappings remain unknown.",
          )
        }}
      </p>
      <div class="notice">
        <ShieldCheck :size="18" />
        <span>
          {{
            preview
              ? t(
                  "Browser preview uses a sample workspace. Launch the desktop app to monitor this computer.",
                )
              : t(
                  "Monitoring runs locally. Close the window to keep watching from the system tray; use Quit AgentLens in the tray menu to exit.",
                )
          }}
        </span>
      </div>
      <div
        v-if="data.warnings.length"
        class="diagnostic-list"
      >
        <h3>{{ t("Collection notes") }}</h3>
        <p
          v-for="w in data.warnings"
          :key="w"
        >
          {{ w }}
        </p>
      </div>
      <template #footer>
        <button
          class="button primary"
          @click="about = false"
        >
          {{ t("Got it") }}
        </button>
      </template>
    </Modal>
  </div>
  <div
    v-else
    class="startup"
  >
    <img
      src="/icon.svg"
      alt="AgentLens"
    />
    <h1>AgentLens</h1>
    <p>{{ error || t("Bringing your workspace into focus…") }}</p>
    <button
      v-if="error"
      class="button secondary"
      @click="reload"
    >
      {{ t("Try again") }}
    </button>
    <LoaderCircle
      v-else
      :size="22"
      class="spin"
    />
  </div>
</template>
