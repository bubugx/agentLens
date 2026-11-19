<script setup lang="ts">
import { computed } from "vue";
import { ChevronLeft, ChevronRight } from "lucide-vue-next";
import { t } from "../lib/i18n";
import UiSelect from "./UiSelect.vue";
const props = defineProps<{
  page: number;
  pageSize: number;
  total: number;
  label: string;
}>();
const emit = defineEmits<{
  "update:page": [number];
  "update:pageSize": [number];
}>();
const pages = computed(() => Math.max(1, Math.ceil(props.total / props.pageSize)));
</script>
<template>
  <nav
    class="list-pagination"
    :aria-label="label"
  >
    <span class="pagination-range">
      {{
        t(
          "{0}–{1} of {2} entries",
          total ? (page - 1) * pageSize + 1 : 0,
          Math.min(total, page * pageSize),
          total,
        )
      }}
    </span>
    <div class="pagination-controls">
      <UiSelect
        :model-value="pageSize"
        :label="t('Entries per page')"
        :options="[10, 20, 50].map((n) => ({ value: n, label: t('{0} per page', n) }))"
        @update:model-value="emit('update:pageSize', Number($event))"
      />
      <span class="pagination-status">{{ t("Page {0} of {1}", page, pages) }}</span>
      <button
        class="icon-button"
        :aria-label="t('Previous page')"
        :disabled="page <= 1"
        @click="emit('update:page', page - 1)"
      >
        <ChevronLeft :size="16" />
      </button>
      <button
        class="icon-button"
        :aria-label="t('Next page')"
        :disabled="page >= pages"
        @click="emit('update:page', page + 1)"
      >
        <ChevronRight :size="16" />
      </button>
    </div>
  </nav>
</template>
