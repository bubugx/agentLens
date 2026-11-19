<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { Settings, CleanupPolicy } from "../lib/types";
import { AGENT_META } from "../lib/types";
import { t } from "../lib/i18n";
import UiSelect from "./UiSelect.vue";
import ContextHelp from "./ContextHelp.vue";
import { Check, X, Plus, RotateCcw } from "lucide-vue-next";
const props = defineProps<{ settings: Settings }>();
const emit = defineEmits<{ update: [settings: Settings] }>();
const clone = (p: CleanupPolicy): CleanupPolicy => JSON.parse(JSON.stringify(p));
const draft = ref(clone(props.settings.cleanup)),
  path = ref(""),
  pathError = ref("");
const dirty = computed(
  () => JSON.stringify(draft.value) !== JSON.stringify(props.settings.cleanup),
);
watch(
  () => JSON.stringify(props.settings.cleanup),
  () => (draft.value = clone(props.settings.cleanup)),
);
function update(key: keyof CleanupPolicy, value: unknown) {
  draft.value = { ...draft.value, preset: "custom", [key]: value };
}
function preset(value: unknown) {
  const name = String(value) as CleanupPolicy["preset"];
  if (name === "custom") {
    update("preset", name);
    return;
  }
  draft.value = {
    ...draft.value,
    preset: name,
    temporaryEnabled: true,
    cacheEnabled: true,
    logsEnabled: true,
    temporaryHours: name === "balanced" ? 6 : 24,
    cacheDays: name === "balanced" ? 3 : 7,
    logDays: name === "balanced" ? 3 : 7,
    sessionCacheEnabled: name === "balanced",
    sessionCacheDays: 7,
    includeSessions: false,
    sessionDays: 30,
    sessionScope: "all",
    keepLatestSessions: 10,
    includeProjectSessions: false,
  };
}
const categories = computed(() => [
  {
    key: "temporaryEnabled" as const,
    days: "temporaryHours" as const,
    label: "Temporary files",
    detail: "Recognized temporary directories; excludes source and configuration.",
    options: [1, 6, 12, 24, 72, 168, 720],
    unit: "{0} hours",
  },
  {
    key: "cacheEnabled" as const,
    days: "cacheDays" as const,
    label: "Application caches",
    detail: "Recognized agent cache directories and editor caches.",
    options: [1, 3, 7, 14, 30, 90, 365],
    unit: "{0} days",
  },
  {
    key: "logsEnabled" as const,
    days: "logDays" as const,
    label: "Logs & diagnostics",
    detail: "Old logs and debug output in recognized agent directories.",
    options: [1, 3, 7, 14, 30, 90, 365],
    unit: "{0} days",
  },
  {
    key: "sessionCacheEnabled" as const,
    days: "sessionCacheDays" as const,
    label: "Session helper caches",
    detail:
      "Codex shell snapshots and Claude paste-cache; conversation transcripts use separate rules.",
    options: [1, 3, 7, 14, 30, 90, 365],
    unit: "{0} days",
  },
]);
function addPath() {
  const value = path.value.trim();
  if (!/^(\/|[a-zA-Z]:[\\/])/.test(value) || value.split(/[\\/]/).includes("..")) {
    pathError.value = t("Enter an absolute path without parent traversal.");
    return;
  }
  if (!draft.value.excludedPaths.includes(value))
    update("excludedPaths", [...draft.value.excludedPaths, value]);
  path.value = "";
  pathError.value = "";
}
function toggleAgent(id: string) {
  update(
    "excludedAgents",
    draft.value.excludedAgents.includes(id)
      ? draft.value.excludedAgents.filter((a) => a !== id)
      : [...draft.value.excludedAgents, id],
  );
}
function apply() {
  if (dirty.value) emit("update", { ...props.settings, cleanup: clone(draft.value) });
}
</script>
<template>
  <section class="panel cleanup-rules">
    <div class="section-heading">
      <div>
        <h2>{{ t("Cleanup rules") }}</h2>
      </div>
      <UiSelect
        :model-value="draft.preset"
        :label="t('Cleanup strategy')"
        :options="[
          { value: 'conservative', label: t('Conservative') },
          { value: 'balanced', label: t('Balanced') },
          { value: 'custom', label: t('Custom rules') },
        ]"
        @update:model-value="preset"
      />
    </div>
    <div class="cleanup-category-grid">
      <div
        v-for="category in categories"
        :key="category.key"
        class="cleanup-rule-card"
      >
        <div class="cleanup-rule-title">
          <strong>{{ t(category.label) }}</strong>
          <ContextHelp :label="t(category.label)">
            <p>{{ t(category.detail) }}</p>
          </ContextHelp>
          <button
            role="switch"
            :aria-label="t(category.label)"
            :aria-checked="draft[category.key]"
            :class="['toggle', { checked: draft[category.key] }]"
            @click="update(category.key, !draft[category.key])"
          >
            <span />
          </button>
        </div>
        <div
          class="cleanup-rule-retention"
          :class="{ inactive: !draft[category.key] }"
        >
          <span>{{ t("Keep at least") }}</span>
          <UiSelect
            :model-value="draft[category.days]"
            :label="t(category.label) + ' · ' + t('Retention')"
            :options="
              category.options.map((n) => ({
                value: n,
                label: t(category.unit, n),
              }))
            "
            @update:model-value="update(category.days, $event)"
          />
        </div>
      </div>
    </div>
    <div class="cleanup-section">
      <div class="cleanup-section-heading">
        <div>
          <h3>{{ t("Conversation history") }}</h3>
          <p>
            {{ t("When enabled, old conversation files can become cleanup candidates.") }}
          </p>
        </div>
        <button
          role="switch"
          :aria-label="t('Include old session histories for manual review')"
          :aria-checked="draft.includeSessions"
          :class="['toggle', { checked: draft.includeSessions }]"
          @click="update('includeSessions', !draft.includeSessions)"
        >
          <span />
        </button>
      </div>
      <div
        class="session-policy-fields"
        :class="{ inactive: !draft.includeSessions }"
      >
        <label>
          <span>{{ t("Session history retention") }}</span>
          <UiSelect
            :model-value="draft.sessionDays"
            :label="t('Session history retention')"
            :options="
              [7, 14, 30, 60, 90, 180, 365].map((n) => ({
                value: n,
                label: t('{0} days', n),
              }))
            "
            @update:model-value="update('sessionDays', $event)"
          />
        </label>
        <label>
          <span>{{ t("Session scope") }}</span>
          <UiSelect
            :model-value="draft.sessionScope"
            :label="t('Session scope')"
            :options="[
              { value: 'all', label: t('All old session files') },
              { value: 'archived', label: t('Archived sessions only') },
            ]"
            @update:model-value="update('sessionScope', $event)"
          />
        </label>
        <label>
          <span>{{ t("Always keep newest") }}</span>
          <UiSelect
            :model-value="draft.keepLatestSessions"
            :label="t('Newest sessions to retain')"
            :options="
              [0, 5, 10, 20, 50, 100, 500, 1000].map((n) => ({
                value: n,
                label: t('{0} session files per directory', n),
              }))
            "
            @update:model-value="update('keepLatestSessions', $event)"
          />
        </label>
      </div>
      <div class="project-session-rule">
        <div>
          <strong>{{ t("Include session histories inside projects") }}</strong>
          <p>
            {{ t("Only recognized .codex / .claude conversation files are included.") }}
          </p>
        </div>
        <button
          role="switch"
          :aria-label="t('Include session histories inside projects')"
          :aria-checked="draft.includeProjectSessions"
          :class="['toggle', { checked: draft.includeProjectSessions }]"
          @click="update('includeProjectSessions', !draft.includeProjectSessions)"
        >
          <span />
        </button>
      </div>
    </div>
    <div class="cleanup-section">
      <h3>{{ t("Agents included in cleanup") }}</h3>

      <div class="cleanup-agent-options">
        <button
          v-for="(meta, id) in AGENT_META"
          v-show="id !== 'unknown' && meta.enabled !== false"
          :key="id"
          :class="['agent-rule-chip', { selected: !draft.excludedAgents.includes(id) }]"
          :aria-pressed="!draft.excludedAgents.includes(id)"
          @click="toggleAgent(id)"
        >
          <span class="rule-check">
            <Check
              v-if="!draft.excludedAgents.includes(id)"
              :size="12"
            />
          </span>
          {{ meta.name }}
        </button>
      </div>
      <div class="rule-minimum">
        <span>{{ t("Minimum candidate size") }}</span>
        <UiSelect
          :model-value="draft.minimumSizeMb"
          :label="t('Minimum candidate size')"
          :options="
            [0, 1, 10, 50, 100, 500, 1000, 10000].map((n) => ({
              value: n,
              label: n ? `${n} MB` : t('Any size'),
            }))
          "
          @update:model-value="update('minimumSizeMb', $event)"
        />
      </div>
      <h3 class="path-exclusions-heading">{{ t("Excluded paths") }}</h3>
      <p class="cleanup-section-description">
        {{ t("Excluded paths and their contents are always kept.") }}
      </p>
      <form
        class="rule-path-entry"
        @submit.prevent="addPath"
      >
        <input
          v-model="path"
          :aria-label="t('Excluded cleanup path')"
          :placeholder="t('Absolute path to keep…')"
        />
        <button
          class="button secondary"
          :disabled="!path.trim() || draft.excludedPaths.length >= 100"
        >
          <Plus :size="14" />
          {{ t("Exclude path") }}
        </button>
      </form>
      <p
        v-if="pathError"
        class="rule-path-error"
        role="alert"
      >
        {{ pathError }}
      </p>
      <div
        v-for="item in draft.excludedPaths"
        :key="item"
        class="rule-excluded-path"
      >
        <code>{{ item }}</code>
        <button
          class="icon-button"
          :aria-label="t('Remove exclusion {0}', item)"
          @click="
            update(
              'excludedPaths',
              draft.excludedPaths.filter((p) => p !== item),
            )
          "
        >
          <X :size="14" />
        </button>
      </div>
    </div>
    <div class="cleanup-rules-save">
      <span role="status">{{ dirty ? t("Unapplied changes") : t("Rules are up to date") }}</span>
      <div class="inline-controls">
        <button
          class="button secondary small"
          :disabled="!dirty"
          @click="draft = clone(settings.cleanup)"
        >
          <RotateCcw :size="13" />
          {{ t("Reset changes") }}
        </button>
        <button
          class="button primary small"
          :disabled="!dirty"
          @click="apply"
        >
          <Check :size="14" />
          {{ t("Apply cleanup rules") }}
        </button>
      </div>
    </div>
  </section>
</template>
