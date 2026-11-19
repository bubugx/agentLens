<script setup lang="ts">
import { ref, watch, computed } from "vue";
import type { Settings } from "../lib/types";
import { t } from "../lib/i18n";
import UiSelect from "./UiSelect.vue";
import { Check } from "lucide-vue-next";
const props = defineProps<{ settings: Settings }>();
const emit = defineEmits<{ update: [settings: Settings] }>();
const accent = ref(props.settings.customAccent);
watch(
  () => props.settings.customAccent,
  (value) => (accent.value = value),
);
const valid = computed(() => /^#[0-9a-f]{6}$/i.test(accent.value));
function update(key: keyof Settings, value: string) {
  emit("update", { ...props.settings, [key]: value });
}
const palettes = [
  { color: "#167c83", name: "Lagoon" },
  { color: "#7160bd", name: "Iris" },
  { color: "#a76b21", name: "Amber" },
  { color: "#b44d76", name: "Rose" },
  { color: "#3273bd", name: "Ocean" },
];
</script>
<template>
  <div class="theme-heading">
    <strong>{{ t("Theme") }}</strong>
  </div>
  <div class="theme-previews">
    <button
      v-for="variant in ['light', 'dark', 'custom']"
      :key="variant"
      :class="['theme-option', variant, { chosen: settings.theme === variant }]"
      :aria-pressed="settings.theme === variant"
      @click="update('theme', variant)"
    >
      <span
        class="theme-sample"
        :style="variant === 'custom' ? { '--sample-accent': settings.customAccent } : {}"
      >
        <i />
        <span>
          <b />
          <b />
          <b />
        </span>
      </span>
      <span>
        {{ t(variant === "light" ? "Light" : variant === "dark" ? "Dark" : "Custom theme") }}
        <Check
          v-if="settings.theme === variant"
          :size="13"
        />
      </span>
    </button>
  </div>
  <div
    v-if="settings.theme === 'custom'"
    class="custom-theme-editor"
  >
    <div class="setting-row">
      <div>
        <strong>{{ t("Base appearance") }}</strong>
      </div>
      <UiSelect
        :model-value="settings.customBase"
        :label="t('Base appearance')"
        :options="[
          { value: 'light', label: t('Light') },
          { value: 'dark', label: t('Dark') },
        ]"
        @update:model-value="update('customBase', $event)"
      />
    </div>
    <div class="accent-editor">
      <label for="theme-accent">{{ t("Accent color") }}</label>
      <div class="color-swatches">
        <button
          v-for="palette in palettes"
          :key="palette.color"
          :style="{ background: palette.color }"
          :aria-label="t(palette.name)"
          :aria-pressed="settings.customAccent === palette.color"
          @click="update('customAccent', palette.color)"
        >
          <Check
            v-if="settings.customAccent === palette.color"
            :size="15"
          />
        </button>
      </div>
      <input
        id="theme-accent"
        v-model="accent"
        class="text-input accent-input"
        :aria-invalid="!valid"
        maxlength="7"
        spellcheck="false"
        placeholder="#167c83"
        @keydown.enter="valid && update('customAccent', accent)"
      />
      <button
        class="button secondary small"
        :disabled="!valid || accent === settings.customAccent"
        @click="update('customAccent', accent)"
      >
        {{ t("Apply color") }}
      </button>
    </div>
  </div>
</template>
