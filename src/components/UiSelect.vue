<script setup lang="ts">
import { computed } from "vue";
import {
  SelectContent,
  SelectItem,
  SelectItemIndicator,
  SelectItemText,
  SelectPortal,
  SelectRoot,
  SelectTrigger,
  SelectViewport,
} from "reka-ui";
import { Check, ChevronDown } from "lucide-vue-next";
const props = defineProps<{
  modelValue: string | number;
  options: { value: string | number; label: string }[];
  label: string;
  id?: string;
  compact?: boolean;
}>();
const emit = defineEmits<{ "update:modelValue": [value: any] }>();
const value = computed(() => String(props.modelValue));
const displayLabel = computed(
  () =>
    props.options.find((o) => String(o.value) === String(props.modelValue))
      ?.label || props.label,
);
function change(value: unknown) {
  const match = props.options.find((o) => String(o.value) === String(value));
  if (match) emit("update:modelValue", match.value);
}
</script>
<template>
  <SelectRoot :model-value="value" @update:model-value="change"
    ><SelectTrigger
      :id="id"
      :aria-label="label"
      :class="['ui-select-trigger', { compact }]"
      ><span>{{ displayLabel }}</span
      ><ChevronDown :size="14" class="select-chevron" /></SelectTrigger
    ><SelectPortal
      ><SelectContent
        class="ui-select-content glass"
        position="popper"
        :side-offset="6"
        align="end"
        :collision-padding="12"
        ><SelectViewport class="ui-select-viewport"
          ><SelectItem
            v-for="option in options"
            :key="option.value"
            :value="String(option.value)"
            class="ui-select-item"
            ><SelectItemText>{{ option.label }}</SelectItemText
            ><SelectItemIndicator class="ui-select-indicator"
              ><Check
                :size="
                  14
                " /></SelectItemIndicator></SelectItem></SelectViewport></SelectContent></SelectPortal
  ></SelectRoot>
</template>
