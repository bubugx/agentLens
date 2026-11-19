<script setup lang="ts">
import { t } from "../lib/i18n";
import { onMounted, onBeforeUnmount, ref } from "vue";
import { X } from "lucide-vue-next";
defineProps<{ title: string; subtitle?: string; wide?: boolean }>();
const emit = defineEmits<{ close: [] }>();
const root = ref<HTMLElement>();
let prior: Element | null = null;
function trap(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
  if (e.key !== "Tab" || !root.value) return;
  const elements = [
    ...root.value.querySelectorAll<HTMLElement>(
      'button:not(:disabled), input, select, [tabindex="0"]',
    ),
  ];
  const first = elements[0],
    last = elements.at(-1);
  if (e.shiftKey && document.activeElement === first) {
    e.preventDefault();
    last?.focus();
  } else if (!e.shiftKey && document.activeElement === last) {
    e.preventDefault();
    first?.focus();
  }
}
onMounted(() => {
  prior = document.activeElement;
  document.addEventListener("keydown", trap);
  root.value?.querySelector<HTMLElement>("input, button")?.focus();
});
onBeforeUnmount(() => {
  document.removeEventListener("keydown", trap);
  if (prior instanceof HTMLElement) prior.focus();
});
</script>
<template>
  <div class="modal-backdrop" @mousedown.self="emit('close')">
    <section
      ref="root"
      :class="['modal', 'glass', { wide }]"
      role="dialog"
      aria-modal="true"
      :aria-label="title"
    >
      <div class="modal-head">
        <div>
          <h2>{{ title }}</h2>
          <p v-if="subtitle">{{ subtitle }}</p>
        </div>
        <button
          class="icon-button"
          :aria-label="t('Close dialog')"
          @click="emit('close')"
        >
          <X :size="18" />
        </button>
      </div>
      <div class="modal-body"><slot /></div>
      <div v-if="$slots.footer" class="modal-footer">
        <slot name="footer" />
      </div>
    </section>
  </div>
</template>
