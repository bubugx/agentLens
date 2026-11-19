<script setup lang="ts">
withDefaults(defineProps<{ tone?: string; dot?: boolean }>(), {
  tone: "neutral",
});
</script>
<template>
  <span :class="['badge', tone]">
    <i v-if="dot" />
    <slot />
  </span>
</template>
