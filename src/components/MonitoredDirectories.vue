<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { ArrowUpRight, FolderSearch, Plus, ScanSearch, Search, X } from "lucide-vue-next";
import { t, rootLabel } from "../lib/i18n";
import { compactPath, type Root } from "../lib/types";
import AgentIcon from "./AgentIcon.vue";
import EmptyState from "./EmptyState.vue";
import ListPagination from "./ListPagination.vue";
import UiTabs from "./UiTabs.vue";
const props = defineProps<{
  roots: Root[];
  automatic: boolean;
  scanning: boolean;
  monitoring: boolean;
}>();
defineEmits<{
  automatic: [boolean];
  scan: [];
  add: [];
  remove: [string];
  reveal: [string];
}>();
const search = ref(""),
  scope = ref("all"),
  page = ref(1),
  pageSize = ref(10);
const files = computed(() =>
  props.roots.filter(
    (r) =>
      (scope.value === "all" || (scope.value === "projects") === r.custom) &&
      `${r.label} ${r.path} ${r.agent}`.toLowerCase().includes(search.value.toLowerCase()),
  ),
);
const visible = computed(() =>
  files.value.slice((page.value - 1) * pageSize.value, page.value * pageSize.value),
);
watch([search, scope, pageSize], () => (page.value = 1));
watch(
  () => files.value.length,
  (n) => (page.value = Math.min(page.value, Math.max(1, Math.ceil(n / pageSize.value)))),
);
const source = (r: Root) =>
  r.source.startsWith("adopted-")
    ? {
        "adopted-project": "Adopted project",
        "adopted-cache": "Adopted cache",
        "adopted-logs": "Adopted logs",
        "adopted-temporary": "Adopted temporary",
      }[r.source] || "Manual"
    : r.source === "scan"
      ? "Found in user space"
      : r.source === "session"
        ? "From session"
        : r.source === "process"
          ? "From process"
          : r.custom
            ? "Manual"
            : "Automatic";
</script>
<template>
  <section class="panel monitored-directories">
    <div class="section-heading">
      <div>
        <h2>
          {{ t("Watched directories") }}
          <span class="count-pill">{{ roots.length }}</span>
        </h2>
      </div>
      <div class="inline-controls">
        <button
          class="button secondary small"
          :disabled="scanning || !monitoring"
          @click="$emit('scan')"
        >
          <ScanSearch :size="14" />
          {{ t("Scan user space") }}
        </button>
        <button
          class="button secondary small"
          @click="$emit('add')"
        >
          <Plus :size="14" />
          {{ t("Add project") }}
        </button>
      </div>
    </div>
    <div class="directory-discovery">
      <span class="directory-discovery-icon"><FolderSearch :size="19" /></span>
      <div>
        <strong>{{ t("Discover projects automatically") }}</strong>
      </div>
      <button
        role="switch"
        :aria-label="t('Discover projects automatically')"
        :aria-checked="automatic"
        :class="['toggle', { checked: automatic }]"
        @click="$emit('automatic', !automatic)"
      >
        <span />
      </button>
    </div>
    <div class="directory-toolbar">
      <UiTabs
        v-model="scope"
        class="filter-tabs"
        :label="t('Directory categories')"
        :options="[
          { value: 'all', label: t('All directories') },
          { value: 'global', label: t('Global agent storage') },
          { value: 'projects', label: t('Project directories') },
        ]"
      />
      <label class="directory-search">
        <Search :size="14" />
        <input
          v-model="search"
          :aria-label="t('Search monitored directories')"
          :placeholder="t('Search directory paths…')"
        />
      </label>
    </div>
    <div class="directory-list">
      <div
        v-for="root in visible"
        :key="root.id"
        class="directory-row"
      >
        <AgentIcon
          :id="root.agent"
          small
        />
        <button
          class="directory-copy"
          @click="$emit('reveal', root.path)"
        >
          <strong>{{ rootLabel(root.label) }}</strong>
          <span :title="root.path">{{ compactPath(root.path) }}</span>
        </button>
        <span class="directory-source">{{ t(source(root)) }}</span>
        <button
          class="icon-button"
          :aria-label="t('Reveal directory {0}', root.label)"
          @click="$emit('reveal', root.path)"
        >
          <ArrowUpRight :size="15" />
        </button>
        <button
          v-if="root.custom"
          class="icon-button directory-remove"
          :aria-label="t('Remove {0}', root.label)"
          @click="$emit('remove', root.id)"
        >
          <X :size="14" />
        </button>
        <span
          v-else
          class="directory-action-placeholder"
        />
      </div>
      <EmptyState
        v-if="!visible.length"
        :title="t('No matching directories')"
        :description="t('Try another path or directory type.')"
      />
    </div>
    <ListPagination
      v-model:page="page"
      v-model:page-size="pageSize"
      :total="files.length"
      :label="t('Directory pagination')"
    />
  </section>
</template>
