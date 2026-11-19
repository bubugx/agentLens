<script setup lang="ts">
import { Folder } from "lucide-vue-next";
defineProps<{ title: string; description: string }>();
</script>
<template>
  <div class="empty-state">
    <div class="empty-icon">
      <slot name="icon"><Folder :size="23" /></slot>
    </div>
    <h3>{{ title }}</h3>
    <p>{{ description }}</p>
    <slot />
  </div>
</template>
