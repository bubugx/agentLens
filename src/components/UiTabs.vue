<script setup lang="ts">
import { nextTick, ref } from "vue";
const props = defineProps<{
  modelValue: string | number | boolean;
  options: {
    value: string | number | boolean;
    label: string;
    count?: number;
    disabled?: boolean;
  }[];
  label: string;
  compact?: boolean;
}>();
const emit = defineEmits<{ "update:modelValue": [value: any] }>();
const root = ref<HTMLElement>();
async function key(event: KeyboardEvent, index: number) {
  if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
  event.preventDefault();
  const enabled = props.options
    .map((o, i) => (o.disabled ? -1 : i))
    .filter((i) => i >= 0);
  const current = enabled.indexOf(index);
  const position =
    event.key === "Home"
      ? 0
      : event.key === "End"
        ? enabled.length - 1
        : (current + (event.key === "ArrowRight" ? 1 : -1) + enabled.length) %
          enabled.length;
  const target = enabled[position];
  if (target === undefined) return;
  emit("update:modelValue", props.options[target].value);
  await nextTick();
  root.value
    ?.querySelectorAll<HTMLButtonElement>('[role="tab"]')
    [target]?.focus();
}
</script>
<template>
  <div
    ref="root"
    :class="['ui-tabs', { compact }]"
    role="tablist"
    :aria-label="label"
    aria-orientation="horizontal"
  >
    <button
      v-for="(option, i) in options"
      :key="String(option.value)"
      type="button"
      role="tab"
      :aria-selected="modelValue === option.value"
      :tabindex="modelValue === option.value ? 0 : -1"
      :disabled="option.disabled"
      :class="{ active: modelValue === option.value }"
      @click="emit('update:modelValue', option.value)"
      @keydown="key($event, i)"
    >
      <span>{{ option.label }}</span><span
        v-if="option.count !== undefined"
        class="tab-count"
        >{{ option.count }}</span
      >
    </button>
  </div>
</template>
