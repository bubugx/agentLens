<script setup lang="ts">
import { ref, useId } from "vue";
import { t, categoryLabel } from "../lib/i18n";
import { ArrowUpRight, Check, Folder, Info } from "lucide-vue-next";
import type { FileEntry } from "../lib/types";
import { bytes, compactPath } from "../lib/types";
defineProps<{ file: FileEntry; checked?: boolean; selectable?: boolean }>();
const emit = defineEmits<{ check: []; reveal: [] }>();
const expanded = ref(false);
const detailsId = useId();
</script>
<template>
  <div class="file-item">
    <div class="file-row">
      <button
        v-if="selectable"
        role="checkbox"
        :aria-checked="checked"
        :aria-label="t('Select {0}', file.relativePath)"
        :class="['checkbox', { selected: checked }]"
        @click="emit('check')"
      >
        <Check
          v-if="checked"
          :size="12"
        />
      </button>
      <span :class="['file-kind', file.category]"><Folder :size="18" /></span>
      <div class="file-description">
        <strong>{{ file.relativePath }}</strong>
        <span :title="file.path">{{ compactPath(file.path.replace(/[\\/][^\\/]+$/, "")) }}</span>
      </div>
      <span class="file-category">{{ categoryLabel(file.category) }}</span>
      <span class="file-bytes tabular">{{ bytes(file.size) }}</span>
      <button
        v-if="file.reason"
        class="icon-button file-details-toggle"
        :aria-label="t('File details for {0}', file.relativePath)"
        :aria-expanded="expanded"
        :aria-controls="detailsId"
        :title="t('File details')"
        @click="expanded = !expanded"
      >
        <Info :size="15" />
      </button>
      <button
        class="icon-button"
        :title="t('Show in folder')"
        :aria-label="t('Reveal {0}', file.relativePath)"
        @click="emit('reveal')"
      >
        <ArrowUpRight :size="16" />
      </button>
    </div>
    <div
      v-if="expanded"
      :id="detailsId"
      class="file-row-details"
    >
      <span :class="{ eligible: file.eligible }">{{ t(file.reason) }}</span>
      <code>{{ file.path }}</code>
    </div>
  </div>
</template>
