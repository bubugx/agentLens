<script setup lang="ts">
import { PopoverRoot, PopoverTrigger, PopoverPortal, PopoverContent, PopoverClose } from "reka-ui";
import { Info, X } from "lucide-vue-next";
import { ref } from "vue";
import { t } from "../lib/i18n";
defineProps<{ label: string }>();
const open = ref(false);
</script>
<template>
  <PopoverRoot v-model:open="open">
    <PopoverTrigger
      class="icon-button context-help"
      :aria-label="label"
    >
      <Info :size="15" />
    </PopoverTrigger>
    <PopoverPortal>
      <PopoverContent
        class="context-help-content"
        side="bottom"
        align="end"
        :side-offset="8"
        :collision-padding="12"
        :aria-label="label"
        @keydown.esc.stop.prevent="open = false"
      >
        <PopoverClose
          class="icon-button"
          :aria-label="t('Close')"
        >
          <X :size="14" />
        </PopoverClose>
        <slot />
      </PopoverContent>
    </PopoverPortal>
  </PopoverRoot>
</template>
