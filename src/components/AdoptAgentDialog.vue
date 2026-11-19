<script setup lang="ts">
import { message } from "../lib/notifications";
import { computed, onMounted, ref, watch } from "vue";
import {
  ArrowLeft,
  Check,
  FolderOpen,
  LoaderCircle,
  Plus,
  RefreshCw,
  Search,
  Terminal,
  X,
} from "lucide-vue-next";
import Modal from "./Modal.vue";
import UiTabs from "./UiTabs.vue";
import UiSelect from "./UiSelect.vue";
import ListPagination from "./ListPagination.vue";
import { t } from "../lib/i18n";
import { bytes, compactPath, AGENT_META } from "../lib/types";
import type {
  AdoptRequest,
  AdoptionPreview,
  ProcessCandidate,
  Snapshot,
  DirectoryBinding,
} from "../lib/types";
import * as client from "../lib/client";
const emit = defineEmits<{ close: []; adopted: [snapshot: Snapshot] }>();
const mode = ref<"process" | "rule">("process"),
  candidates = ref<ProcessCandidate[]>([]),
  query = ref(""),
  loading = ref(false),
  busy = ref(false),
  error = ref(""),
  review = ref<AdoptionPreview | null>(null),
  candidatePage = ref(1),
  directoryPath = ref(""),
  directoryUsage = ref<DirectoryBinding["usage"]>("project");
const request = ref<AdoptRequest>({
  name: "",
  color: "#528f80",
  pid: null,
  startTime: null,
  executable: "",
  entrypoint: "",
  remember: true,
  directories: [],
});
const selected = computed(() =>
  candidates.value.find(
    (c) => c.pid === request.value.pid && c.startTime === request.value.startTime,
  ),
);
const filtered = computed(() =>
  candidates.value.filter((c) =>
    `${c.name} ${c.pid} ${c.executable} ${c.entrypoint}`
      .toLowerCase()
      .includes(query.value.toLowerCase()),
  ),
);
const pageSize = ref(10);
const ready = computed(
  () =>
    request.value.name.trim() &&
    request.value.executable &&
    (mode.value === "rule" || selected.value),
);
const colors = ["#528f80", "#8584b9", "#167c83", "#b36e46", "#627aaf", "#a85b83"];
watch(query, () => (candidatePage.value = 1));
async function load() {
  loading.value = true;
  error.value = "";
  try {
    candidates.value = await client.listProcessCandidates();
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}
onMounted(load);
function select(c: ProcessCandidate) {
  if (c.agent) return;
  request.value = {
    ...request.value,
    pid: c.pid,
    startTime: c.startTime,
    executable: c.executable,
    entrypoint: c.entrypoint,
    remember: c.canRemember,
    name:
      request.value.name ||
      (c.entrypoint.startsWith("module:")
        ? c.entrypoint.slice(7)
        : c.workingDirectory.split(/[\\/]/).pop() || c.name),
  };
  error.value = "";
}
function changeMode(value: "process" | "rule") {
  mode.value = value;
  review.value = null;
  request.value = {
    ...request.value,
    pid: null,
    startTime: null,
    executable: "",
    entrypoint: "",
    remember: true,
  };
  error.value = "";
}
async function browse() {
  if (!client.native) {
    directoryPath.value = "/Users/alex/Developer/research-agent";
    return;
  }
  const { open } = await import("@tauri-apps/plugin-dialog");
  try {
    const path = await open({
      directory: true,
      multiple: false,
      title: t("Bind monitoring directory"),
    });
    if (typeof path === "string") directoryPath.value = path;
  } catch (e) {
    error.value = String(e);
  }
}
function addDirectory() {
  const path = directoryPath.value.trim().replace(/[\\/]$/, "");
  if (!path) return;
  if (request.value.directories.some((d) => d.path === path)) {
    error.value = t("This directory overlaps an existing monitoring root");
    return;
  }
  request.value.directories.push({ path, usage: directoryUsage.value });
  directoryPath.value = "";
  error.value = "";
}
async function preview() {
  busy.value = true;
  error.value = "";
  try {
    review.value = await client.previewAdoption(JSON.parse(JSON.stringify(request.value)));
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}
async function confirm() {
  busy.value = true;
  error.value = "";
  try {
    emit("adopted", await client.adoptAgent(JSON.parse(JSON.stringify(request.value))));
  } catch (e) {
    error.value = String(e);
    review.value = null;
  } finally {
    busy.value = false;
  }
}
const usageOptions = computed(() => [
  { value: "project", label: t("Project · monitor only") },
  { value: "cache", label: t("Cache") },
  { value: "logs", label: t("Logs") },
  { value: "temporary", label: t("Temporary") },
]);
watch(error, (value) => {
  if (value) message(t(value.replace(/^Error: /, "")), "error");
});
</script>
<template>
  <Modal
    :title="t('Adopt Agent')"
    :subtitle="t('Connect any Agent through its running process or an exact recognition rule.')"
    wide
    @close="!busy && emit('close')"
  >
    <div class="adoption-flow">
      <template v-if="!review">
        <UiTabs
          :model-value="mode"
          :label="t('Adoption source')"
          :options="[
            { value: 'process', label: t('Running process') },
            { value: 'rule', label: t('Recognition rule') },
          ]"
          @update:model-value="changeMode($event)"
        />
        <section
          v-if="mode === 'process'"
          class="adoption-candidates"
        >
          <div class="adoption-toolbar">
            <label class="adoption-search">
              <Search :size="15" />
              <input
                v-model="query"
                :placeholder="t('Search process, PID, or executable')"
                :aria-label="t('Search process, PID, or executable')"
              />
            </label>
            <button
              class="button"
              :disabled="loading"
              @click="load"
            >
              <RefreshCw
                :size="15"
                :class="{ spin: loading }"
              />
              {{ t("Refresh") }}
            </button>
          </div>
          <p class="adoption-hint">
            {{
              t(
                "Only your user’s processes are listed. AgentLens and its launch chain are excluded.",
              )
            }}
          </p>
          <div
            v-if="loading"
            class="adoption-empty"
            role="status"
          >
            <LoaderCircle
              :size="18"
              class="spin"
            />
            {{ t("Loading processes…") }}
          </div>
          <div
            v-else
            class="adoption-process-list"
            role="group"
            :aria-label="t('Available processes')"
          >
            <button
              v-for="c in filtered.slice((candidatePage - 1) * pageSize, candidatePage * pageSize)"
              :key="`${c.pid}:${c.startTime}`"
              :disabled="!!c.agent"
              :class="['adoption-process', { chosen: selected?.pid === c.pid }]"
              :aria-pressed="selected?.pid === c.pid"
              @click="select(c)"
            >
              <span class="adoption-process-check">
                <Check
                  v-if="selected?.pid === c.pid"
                  :size="14"
                />
                <Terminal
                  v-else
                  :size="15"
                />
              </span>
              <span class="adoption-process-info">
                <strong>
                  {{ c.name }}
                  <small>PID {{ c.pid }}</small>
                </strong>
                <span :title="c.entrypoint || c.executable">
                  {{ compactPath(c.entrypoint || c.executable) }}
                </span>
              </span>
              <span class="adoption-process-metrics">
                <small v-if="c.agent">
                  {{ AGENT_META[c.agent]?.name || c.agent }} · {{ t("Managed") }}
                </small>
                <template v-else>
                  <strong>{{ bytes(c.memory) }}</strong>
                  <small>CPU {{ c.cpu.toFixed(1) }}%</small>
                </template>
              </span>
            </button>
            <p
              v-if="!filtered.length"
              class="adoption-empty"
            >
              {{ t("No matching processes") }}
            </p>
          </div>
          <ListPagination
            v-if="filtered.length > pageSize"
            :page="candidatePage"
            :page-size="pageSize"
            :total="filtered.length"
            :label="t('Processes')"
            @update:page="candidatePage = $event"
            @update:page-size="
              pageSize = $event;
              candidatePage = 1;
            "
          />
        </section>
        <div class="adoption-identity">
          <label class="adoption-field">
            <span>{{ t("Agent name") }}</span>
            <input
              v-model="request.name"
              maxlength="64"
              :placeholder="t('For example, Research Agent')"
            />
          </label>
          <div class="adoption-field">
            <span>{{ t("Color") }}</span>
            <div
              class="adoption-colors"
              role="group"
              :aria-label="t('Agent color')"
            >
              <button
                v-for="color in colors"
                :key="color"
                :style="{ '--swatch': color }"
                :aria-label="color"
                :aria-pressed="request.color === color"
                @click="request.color = color"
              >
                <Check
                  v-if="request.color === color"
                  :size="14"
                />
              </button>
            </div>
          </div>
        </div>
        <template v-if="mode === 'rule'">
          <label class="adoption-field">
            <span>{{ t("Executable path") }}</span>
            <input
              v-model="request.executable"
              placeholder="/absolute/path/to/agent"
            />
          </label>
          <label class="adoption-field">
            <span>{{ t("Script or module entry point") }}</span>
            <input
              v-model="request.entrypoint"
              placeholder="/absolute/path/agent.py · module:my_agent"
            />
          </label>
          <p class="adoption-hint">
            {{
              t(
                "Desktop apps do not need a CLI. Use the executable inside the app; leave the script entry point empty.",
              )
            }}
          </p>
          <p class="adoption-hint">
            {{
              t(
                "Match the exact executable. Shared runtimes such as Node and Python also require an absolute script path or module:package.name.",
              )
            }}
          </p>
        </template>
        <div
          v-else-if="selected"
          class="adoption-remember"
        >
          <div>
            <strong>{{ t("Remember recognition rule") }}</strong>
            <p>
              {{
                selected.canRemember
                  ? t("Automatically adopt future runs with the same executable and entry point.")
                  : t(
                      "This invocation has no stable script entry point. Adopt this process tree for this app session only.",
                    )
              }}
            </p>
            <code v-if="request.remember">
              {{ compactPath(request.executable) }}
              <br v-if="request.entrypoint" />
              {{ request.entrypoint }}
            </code>
          </div>
          <button
            class="toggle"
            role="switch"
            :aria-label="t('Remember recognition rule')"
            :aria-checked="request.remember"
            :class="{ checked: request.remember }"
            :disabled="!selected.canRemember"
            @click="request.remember = !request.remember"
          >
            <span />
          </button>
        </div>
        <section class="adoption-directories">
          <div class="adoption-section-title">
            <h3>{{ t("Bound directories") }}</h3>
            <span>{{ t("Optional") }}</span>
          </div>
          <p class="adoption-hint">
            {{
              t(
                "Bind project folders for statistics, or explicitly mark cache, log and temporary folders for cleanup review.",
              )
            }}
          </p>
          <div class="adoption-directory-entry">
            <div class="adoption-path">
              <input
                v-model="directoryPath"
                :aria-label="t('Directory path')"
                :placeholder="t('Directory path')"
                @keydown.enter.prevent="addDirectory"
              />
              <button
                class="icon-button"
                :aria-label="t('Choose directory')"
                @click="browse"
              >
                <FolderOpen :size="17" />
              </button>
            </div>
            <UiSelect
              v-model="directoryUsage"
              :options="usageOptions"
              :label="t('Directory usage')"
            />
            <button
              class="button"
              :disabled="!directoryPath.trim() || request.directories.length >= 20"
              @click="addDirectory"
            >
              <Plus :size="16" />
              {{ t("Add") }}
            </button>
          </div>
          <div
            v-for="(d, i) in request.directories"
            :key="d.path"
            class="adoption-bound-row"
          >
            <span :title="d.path">{{ compactPath(d.path) }}</span>
            <small>{{ usageOptions.find((o) => o.value === d.usage)?.label }}</small>
            <button
              class="icon-button"
              :aria-label="t('Remove directory')"
              @click="request.directories.splice(i, 1)"
            >
              <X :size="15" />
            </button>
          </div>
        </section>
      </template>
      <template v-else>
        <div class="adoption-preview-summary">
          <span
            class="adoption-avatar"
            :style="{ background: request.color }"
          >
            {{ request.name.slice(0, 1).toUpperCase() }}
          </span>
          <div>
            <h3>{{ request.name }}</h3>
            <p>
              {{ request.remember ? t("Saved recognition rule") : t("Current app session only") }}
            </p>
          </div>
        </div>
        <div class="adoption-preview-totals">
          <div>
            <span>{{ t("Processes") }}</span>
            <strong>{{ review.processes.length }}</strong>
          </div>
          <div>
            <span>{{ t("Memory") }}</span>
            <strong>{{ bytes(review.processes.reduce((s, p) => s + p.memory, 0)) }}</strong>
          </div>
          <div>
            <span>{{ t("Bound directories") }}</span>
            <strong>{{ review.roots.length }}</strong>
          </div>
        </div>
        <p class="adoption-hint">
          {{
            t(
              "Process descendants are followed automatically. Ownership describes the configured process tree; it does not prove which process wrote each file or reconstruct historical sessions.",
            )
          }}
        </p>
        <div class="adoption-review-list">
          <div
            v-for="p in review.processes"
            :key="p.pid"
            class="adoption-review-row"
          >
            <Terminal :size="16" />
            <span>
              <strong>{{ p.name }} · PID {{ p.pid }}</strong>
              <small>{{ compactPath(p.executable) }}</small>
            </span>
            <b>{{ bytes(p.memory) }}</b>
          </div>
          <p
            v-if="!review.processes.length"
            class="adoption-hint"
          >
            {{ t("No running matches. The rule will take effect when a matching process starts.") }}
          </p>
        </div>
        <div
          v-for="root in review.roots"
          :key="root.id"
          class="adoption-bound-row"
        >
          <FolderOpen :size="15" />
          <span>{{ compactPath(root.path) }}</span>
          <small>{{ usageOptions.find((o) => `adopted-${o.value}` === root.source)?.label }}</small>
        </div>
        <div class="notice">
          {{
            t(
              "Project files and unknown conversation histories remain protected. Cache, log and temporary folders follow your cleanup settings, with active-process and file-identity checks.",
            )
          }}
        </div>
      </template>
    </div>
    <template #footer>
      <button
        class="button"
        :disabled="busy"
        @click="review ? (review = null) : emit('close')"
      >
        <ArrowLeft
          v-if="review"
          :size="15"
        />
        {{ review ? t("Back") : t("Cancel") }}
      </button>
      <button
        v-if="!review"
        class="button primary"
        :disabled="!ready || busy || loading"
        @click="preview"
      >
        <LoaderCircle
          v-if="busy"
          :size="15"
          class="spin"
        />
        {{ t("Preview adoption") }}
      </button>
      <button
        v-else
        class="button primary"
        :disabled="busy"
        @click="confirm"
      >
        <LoaderCircle
          v-if="busy"
          :size="15"
          class="spin"
        />
        {{ t("Confirm adoption") }}
      </button>
    </template>
  </Modal>
</template>
