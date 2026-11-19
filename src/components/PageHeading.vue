<script setup lang="ts">
defineProps<{ title: string }>();
</script>
<template>
  <div class="page-heading subpage-heading">
    <div>
      <h1>{{ title }}</h1>
    </div>
    <div
      v-if="$slots.default"
      class="page-heading-actions"
    >
      <slot />
    </div>
  </div>
</template>
