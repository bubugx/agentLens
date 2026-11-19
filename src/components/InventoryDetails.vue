<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import {
  ArrowLeft,
  ChevronLeft,
  ChevronRight,
  Folder,
  LoaderCircle,
  Search,
  RotateCw,
} from "lucide-vue-next";
import { t, rootLabel } from "../lib/i18n";
import { bytes, compactPath, AGENT_META, type InventoryDetails } from "../lib/types";
import * as client from "../lib/client";
import { message } from "../lib/notifications";
import Modal from "./Modal.vue";
import FileRow from "./FileRow.vue";
const props = defineProps<{
  status: string;
  label: string;
  scanning: boolean;
}>();
defineEmits<{ close: [] }>();
const data = ref<InventoryDetails | null>(null),
  root = ref<string | null>(null),
  query = ref(""),
  offset = ref(0),
  loading = ref(true),
  error = ref("");
let generation = 0,
  timer: ReturnType<typeof setTimeout> | undefined;
const group = computed(() => data.value?.groups.find((g) => g.id === root.value));
const groupTotal = computed(() => data.value?.groups.reduce((s, g) => s + g.bytes, 0) || 0);
async function load() {
  const request = ++generation;
  loading.value = true;
  error.value = "";
  try {
    const result = await client.inventoryDetails(
      props.status,
      root.value,
      query.value,
      offset.value,
    );
    if (request === generation) data.value = result;
  } catch (e) {
    if (request === generation) {
      error.value = String(e);
      message(t(error.value), "error", {
        action: load,
        actionLabel: t("Try again"),
      });
    }
  } finally {
    if (request === generation) loading.value = false;
  }
}
watch([() => props.status, root, offset], load, { immediate: true });
watch(query, () => {
  ++generation;
  loading.value = true;
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => {
    if (offset.value) offset.value = 0;
    else load();
  }, 250);
});
function select(id: string | null) {
  offset.value = 0;
  root.value = id;
}
onBeforeUnmount(() => {
  ++generation;
  if (timer) clearTimeout(timer);
});
</script>
<template>
  <Modal
    :title="t('{0} · storage distribution', label)"
    wide
    @close="$emit('close')"
  >
    <div class="inventory-summary">
      <div>
        <span>{{ t("Category footprint") }}</span>
        <strong>{{ data ? bytes(data.bytes) : "—" }}</strong>
      </div>
      <div>
        <span>{{ t("Files") }}</span>
        <strong>{{ data?.count.toLocaleString() ?? "—" }}</strong>
      </div>
      <div>
        <span>{{ t("Projects / directories") }}</span>
        <strong>{{ root ? 1 : (data?.groups.length ?? "—") }}</strong>
      </div>
    </div>
    <p
      v-if="scanning"
      class="inventory-notice"
    >
      {{
        t(
          "A scan is running. This view shows the last committed inventory. Refresh after the scan finishes.",
        )
      }}
    </p>
    <div class="inventory-toolbar">
      <button
        v-if="root"
        class="text-button"
        @click="select(null)"
      >
        <ArrowLeft :size="14" />
        {{ t("All directories") }}
      </button>
      <span v-else>{{ t("Distribution by project / directory") }}</span>
      <label class="inventory-search">
        <Search :size="14" />
        <input
          v-model="query"
          :aria-label="t('Search inventory files')"
          :placeholder="t('Search file paths…')"
        />
      </label>
      <button
        class="icon-button"
        :aria-label="t('Refresh distribution')"
        :disabled="loading"
        @click="load"
      >
        <RotateCw :size="14" />
      </button>
    </div>
    <div
      class="inventory-content"
      :aria-busy="loading"
    >
      <div
        v-if="loading && !data"
        class="inventory-empty"
      >
        <LoaderCircle
          class="spin"
          :size="23"
        />
        {{ t("Loading file distribution…") }}
      </div>
      <template v-else-if="data">
        <div
          v-if="!root && data.groups.length"
          class="inventory-groups"
        >
          <button
            v-for="g in data.groups"
            :key="g.id"
            class="inventory-group"
            :aria-label="t('Inspect directory {0}', g.label)"
            @click="select(g.id)"
          >
            <span class="inventory-folder"><Folder :size="17" /></span>
            <span class="inventory-group-copy">
              <strong>
                {{ rootLabel(g.label) }}
                <small>{{ t(AGENT_META[g.agent]?.name || g.agent) }}</small>
              </strong>
              <span :title="g.path">{{ compactPath(g.path) }}</span>
              <i>
                <b
                  :style="{
                    width: `${Math.max(1, (g.bytes / Math.max(1, groupTotal)) * 100)}%`,
                  }"
                />
              </i>
            </span>
            <span class="inventory-group-size">
              <strong>{{ bytes(g.bytes) }}</strong>
              <small>
                {{ t("{0} files", g.count.toLocaleString()) }} ·
                {{ ((g.bytes / Math.max(1, groupTotal)) * 100).toFixed(1) }}%
              </small>
            </span>
            <ChevronRight :size="15" />
          </button>
        </div>
        <div class="inventory-file-heading">
          <strong>{{ group ? rootLabel(group.label) : t("Files in this category") }}</strong>
          <span>{{ t("Largest first") }}</span>
        </div>
        <div
          v-if="data.files.length"
          class="inventory-files"
        >
          <FileRow
            v-for="file in data.files"
            :key="file.id"
            :file="file"
            @reveal="client.reveal(file.path).catch((e) => (error = String(e)))"
          />
        </div>
        <div
          v-else
          class="inventory-empty"
        >
          {{ t("No files match this filter.") }}
        </div>
        <div class="inventory-pagination">
          <span>
            {{
              t(
                "{0}–{1} of {2} files",
                data.count ? offset + 1 : 0,
                Math.min(offset + data.limit, data.count),
                data.count.toLocaleString(),
              )
            }}
          </span>
          <button
            class="button secondary small"
            :disabled="offset === 0 || loading"
            :aria-label="t('Previous page')"
            @click="offset = Math.max(0, offset - 50)"
          >
            <ChevronLeft :size="14" />
          </button>
          <button
            class="button secondary small"
            :disabled="offset + data.limit >= data.count || loading"
            :aria-label="t('Next page')"
            @click="offset += 50"
          >
            <ChevronRight :size="14" />
          </button>
        </div>
      </template>
    </div>
    <template #footer>
      <button
        class="button secondary"
        @click="$emit('close')"
      >
        {{ t("Close") }}
      </button>
    </template>
  </Modal>
</template>
